use crate::config;
use crate::key_mouse;
use crate::screen::Cap;
use enigo::Enigo;
use enigo::KeyboardControllable;
use enigo::MouseControllable;
use flate2::Compression;
use flate2::write::ZlibEncoder;
use websocket::OwnedMessage;
use websocket::sync::Reader;
use websocket::sync::Server;
use websocket::sync::Writer;
use std::io::Write;
use std::net::Ipv4Addr;
use std::net::SocketAddrV4;
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

pub fn run(port: u16) {
    let host = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port);
    let server = Server::bind(host).unwrap();
    for request in server.filter_map(Result::ok) {
        if !request.protocols().contains(&"diffscreen".to_string()) {
            request.reject().unwrap();
            continue;
        }
        println!("New income !");
        let client = request.use_protocol("diffscreen").accept().unwrap();
        let (receiver, sender) = client.split().unwrap();
        let running = Arc::new(AtomicBool::new(true));
        let th1run = running.clone();
        let _ = std::thread::spawn(move || {
            screen_stream(sender, th1run);
        });
        let th = std::thread::spawn(move || {
            event(receiver);
        });
        let _ = th.join();
        running.store(false, Ordering::Relaxed);
        println!("Break !");
    }
}

/**
 * 事件处理
 */
fn event(mut stream: Reader<TcpStream>) {
    let mut enigo = Enigo::new();
    for message in stream.incoming_messages() {
        let message = match message {
            Ok(message) => message,
            Err(e) => {
                eprintln!("Msg err {}", e);
                continue;
            }
        };
        match message {
            OwnedMessage::Binary(cmd) => {
                match cmd[0] {
                    dscom::KEY_UP => {
                        if let Some(key) = key_mouse::key_to_enigo(cmd[1]) {
                            enigo.key_up(key);
                        }
                    }
                    dscom::KEY_DOWN => {
                        if let Some(key) = key_mouse::key_to_enigo(cmd[1]) {
                            enigo.key_down(key);
                        }
                    }
                    dscom::MOUSE_KEY_UP => {
                        if let Some(key) = key_mouse::mouse_to_engin(cmd[1]) {
                            enigo.mouse_up(key);
                        }
                    }
                    dscom::MOUSE_KEY_DOWN => {
                        if let Some(key) = key_mouse::mouse_to_engin(cmd[1]) {
                            enigo.mouse_down(key);
                        }
                    }
                    dscom::MOUSE_WHEEL_UP => {
                        enigo.mouse_scroll_y(-2);
                    }
                    dscom::MOUSE_WHEEL_DOWN => {
                        enigo.mouse_scroll_y(2);
                    }
                    dscom::MOVE => {
                        let x = ((cmd[1] as i32) << 8) | (cmd[2] as i32);
                        let y = ((cmd[3] as i32) << 8) | (cmd[4] as i32);
                        enigo.mouse_move_to(x, y);
                    }
                    _ => {
                        return;
                    }
                }
            }
            OwnedMessage::Close(_) => {
                println!("Front close !");
                return;
            }
            _=> {
            }
        }
        
    }
}

fn get_data(data: OwnedMessage) -> Vec<u8> {
    if let OwnedMessage::Binary(x) = data {
        return x;
    }
    return Vec::new();
}

/// 初始化rgb块
fn get_rgb_block(w: usize, h: usize, sw: usize, sh: usize, offset: usize) -> (Vec<Vec<u8>>, usize, usize) {
    let srw = (w / sw) + if w % sw == 0 {0usize} else {1usize};
    let srh = (h / sh) + if h % sh == 0 {0usize} else {1usize};
    let num = srw * srh;
    let mut buf = vec![vec![0u8;offset + sw*sh*3];num];
    for i in 0..(srw * srh) {
        buf[i][offset - 2] = (i >> 8) as u8;
        buf[i][offset - 1] = i as u8;
    }
    (buf, srw, srh)
}

fn ws_send(mut stream: Writer<TcpStream>, data: Vec<u8>) -> (Writer<TcpStream>, Vec<u8>) {
    let om = OwnedMessage::Binary(data);
    stream.send_message(&om).unwrap();
    return (stream, get_data(om))
}

fn screen_stream(mut stream: Writer<TcpStream>, running: Arc<AtomicBool>) {
    let mut cap = Cap::new(config::SW, config::SH, 2);
    let (w, h, sw, sh, _) = cap.size_info();
    let (mut a, srw, srh) = get_rgb_block(w, h, sw, sh, 2);
    let block_len = srw * srh;
    let (mut b, _, _) = get_rgb_block(w, h, sw, sh, 2);
    // 发送w, h, sw, sh
    let mut meta = vec![0u8;8];
    meta[0] = (w >> 8) as u8;
    meta[1] = w as u8;
    meta[2] = (h >> 8) as u8;
    meta[3] = h as u8;
    meta[4] = (sw >> 8) as u8;
    meta[5] = sw as u8;
    meta[6] = (sh >> 8) as u8;
    meta[7] = sh as u8;
    (stream, _) = ws_send(stream, meta);
    let mut sendbuf = Vec::<u8>::with_capacity(sw * sh * 3);
    while running.load(Ordering::Relaxed) {
        cap.cap(&mut b);
        // 对比a
        for i in 0..block_len {
            let (a, b) = (&mut a[i], &b[i]);
            if a != b {
                unsafe {
                    sendbuf.set_len(0);
                }
                a[2..].iter_mut().zip(b[2..].iter()).for_each(|(x, y)|{
                    *x ^= *y;
                });
                let mut e = ZlibEncoder::new(sendbuf, Compression::default());
                e.write_all(&a).unwrap();
                sendbuf = e.finish().unwrap();
                (stream, sendbuf) = ws_send(stream, sendbuf);
            }
        }
        (a, b) = (b, a);
    }
}
