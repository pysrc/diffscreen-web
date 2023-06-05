use crate::key_mouse;
use crate::screen::Cap;
use enigo::Enigo;
use enigo::KeyboardControllable;
use enigo::MouseControllable;
use flate2::Compression;
use flate2::write::DeflateEncoder;
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
use rayon::prelude::*;

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


/*
a: 老图像
b: 新图像
return: 老图像, 待发送图像
 */
#[inline]
fn cap_and_swap(mut enzip: DeflateEncoder<Vec<u8>>, mut cap: Cap, mut a: Vec<u8>, mut b: Vec<u8>) -> (DeflateEncoder<Vec<u8>>, Cap, Vec<u8>, Vec<u8>) {
    loop {
        cap.cap(&mut b);
        if a == b {
            continue;
        }
        // 计算差异
        a.par_iter_mut().zip(b.par_iter()).for_each(|(d1, d2)|{
            *d1 ^= *d2;
        });
        // 压缩
        enzip.write_all(&mut a).unwrap();
        unsafe {
            a.set_len(0);
        }
        let c = enzip.reset(a).unwrap();
        return (enzip, cap, b, c);
    }
}

fn get_data(data: OwnedMessage) -> Vec<u8> {
    if let OwnedMessage::Binary(x) = data {
        return x;
    }
    return Vec::new();
}

fn ws_send(mut stream: Writer<TcpStream>, data: Vec<u8>) -> (Writer<TcpStream>, Vec<u8>) {
    let om = OwnedMessage::Binary(data);
    stream.send_message(&om).unwrap();
    return (stream, get_data(om))
}

fn screen_stream(mut stream: Writer<TcpStream>, running: Arc<AtomicBool>) {
    let mut cap = Cap::new();
    let (w, h) = cap.wh();
    let dlen = w * h * 3;
    let mut a = Vec::<u8>::with_capacity(dlen);
    let b: Vec<u8> = Vec::<u8>::with_capacity(dlen);
    let c = Vec::<u8>::with_capacity(dlen);
    let mut enzip = DeflateEncoder::new(c, Compression::default());

    // 发送w, h
    let mut meta = vec![0u8;4];
    meta[0] = (w >> 8) as u8;
    meta[1] = w as u8;
    meta[2] = (h >> 8) as u8;
    meta[3] = h as u8;
    (stream, _) = ws_send(stream, meta);
    
    // 第一帧
    unsafe {
        a.set_len(dlen);
    }
    cap.cap(&mut a);
    // 压缩
    enzip.write_all(&a).unwrap();
    let mut b = enzip.reset(b).unwrap();
    (stream, b) = ws_send(stream, b);
    while running.load(Ordering::Relaxed) {
        unsafe {
            b.set_len(dlen);
        }
        (enzip, cap, a, b) = cap_and_swap(enzip, cap, a, b);
        (stream, b) = ws_send(stream, b);
    }
}
