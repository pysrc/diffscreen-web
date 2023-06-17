use crate::config;
use crate::key_mouse;
use crate::screen::Cap;
use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;
use enigo::Enigo;
use enigo::KeyboardControllable;
use enigo::MouseControllable;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use websocket::sync::Client;
use std::fs;
use std::io::Write;
use std::net::Ipv4Addr;
use std::net::SocketAddrV4;
use std::net::TcpStream;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use websocket::sync::Reader;
use websocket::sync::Server;
use websocket::sync::Writer;
use websocket::OwnedMessage;

pub fn run(port: u16) {
    let host = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port);
    let server = Server::bind(host).unwrap();
    let isctrl = Arc::new(AtomicBool::new(false));
    for request in server.filter_map(Result::ok) {
        if request.protocols().len() != 1 {
            request.reject().unwrap();
            continue;
        }
        let proto = request.protocols()[0].as_str();
        match proto {
            "diffscreen" => {
                if isctrl.load(Ordering::Relaxed) {
                    request.reject().unwrap();
                    continue;
                }
                println!("New income !");
                let client = request.use_protocol("diffscreen").accept().unwrap();
                let (receiver, sender) = client.split().unwrap();
                isctrl.store(true, Ordering::Relaxed);
                let isctrlc = isctrl.clone();
                let _ = std::thread::spawn(move || {
                    screen_stream(sender, isctrlc);
                });
                let isctrlc = isctrl.clone();
                let _ = std::thread::spawn(move || {
                    event(receiver);
                    isctrlc.store(false, Ordering::Relaxed);
                    println!("Break !");
                });
            }
            "diffscreen-transfer" => {
                // 处理复制粘贴、文件
                let client = request.use_protocol("diffscreen-transfer").accept().unwrap();
                std::thread::spawn(move ||{
                    handle_transfer(client);
                });
            }
            _ => {
                request.reject().unwrap();
                continue;
            }
        }
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
                return;
            }
        };
        match message {
            OwnedMessage::Binary(cmd) => match cmd[0] {
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
            },
            OwnedMessage::Close(_) => {
                println!("Front close !");
                return;
            }
            _ => {}
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
fn get_rgb_block(
    w: usize,
    h: usize,
    sw: usize,
    sh: usize,
    offset: usize,
) -> (Vec<Vec<u8>>, usize, usize) {
    let srw = (w / sw) + if w % sw == 0 { 0usize } else { 1usize };
    let srh = (h / sh) + if h % sh == 0 { 0usize } else { 1usize };
    let num = srw * srh;
    let mut buf = vec![vec![255u8; offset + sw * sh * 3]; num];
    for i in 0..(srw * srh) {
        buf[i][offset - 2] = (i >> 8) as u8;
        buf[i][offset - 1] = i as u8;
    }
    (buf, srw, srh)
}

fn ws_send(mut stream: Writer<TcpStream>, data: Vec<u8>) -> (Writer<TcpStream>, Vec<u8>) {
    let om = OwnedMessage::Binary(data);
    stream.send_message(&om).unwrap();
    return (stream, get_data(om));
}

fn screen_stream(mut stream: Writer<TcpStream>, running: Arc<AtomicBool>) {
    let mut cap = Cap::new(config::SW, config::SH, 2);
    let (w, h, sw, sh, _) = cap.size_info();
    let (mut a, srw, srh) = get_rgb_block(w, h, sw, sh, 2);
    let block_len = srw * srh;
    let (mut b, _, _) = get_rgb_block(w, h, sw, sh, 2);
    // 发送w, h, sw, sh, mask
    let mut meta = vec![0u8; 9];
    meta[0] = (w >> 8) as u8;
    meta[1] = w as u8;
    meta[2] = (h >> 8) as u8;
    meta[3] = h as u8;
    meta[4] = (sw >> 8) as u8;
    meta[5] = sw as u8;
    meta[6] = (sh >> 8) as u8;
    meta[7] = sh as u8;
    meta[8] = config::BIT_MASK;
    (stream, _) = ws_send(stream, meta);
    let mut sendbuf = Vec::<u8>::with_capacity(1024 * 4);
    while running.load(Ordering::Relaxed) {
        cap.cap(&mut b);
        // 对比a
        unsafe {
            sendbuf.set_len(0);
        }
        let mut e = DeflateEncoder::new(sendbuf, Compression::default());
        for i in 0..block_len {
            let (a, b) = (&a[i], &b[i]);
            if a != b {
                e.write_all(&b).unwrap();
            }
        }
        sendbuf = e.finish().unwrap();
        if sendbuf.len() > 0 {
            (stream, sendbuf) = ws_send(stream, sendbuf);
        }
        (a, b) = (b, a);
    }
}

fn get_files(dir: &str) -> Vec<String> {
    let mut res = Vec::<String>::new();
    let mt = fs::metadata(dir);
    if mt.is_err() {
        fs::create_dir(dir).unwrap();
        return res;
    }
    let folder = fs::read_dir(dir).unwrap();
    for file in folder {
        let f = file.unwrap();
        if f.file_type().unwrap().is_file() {
            res.push(f.path().file_name().unwrap().to_string_lossy().to_string());
        }
    }
    return res;
}

fn handle_transfer(client: Client<TcpStream>) {
    let (mut receiver, mut sender) = client.split().unwrap();
    let mut cbctx: ClipboardContext = ClipboardProvider::new().unwrap();
    for message in receiver.incoming_messages() {
        let message = match message {
            Ok(message) => message,
            Err(e) => {
                eprintln!("Msg err {}", e);
                return;
            }
        };
        match message {
            OwnedMessage::Text(ctx) => {
                if ctx.starts_with("paste-text") {
                    let ctx = ctx.replacen("paste-text ", "", 1);
                    cbctx.set_contents(ctx).unwrap();
                } else if ctx.starts_with("copy-text") {
                    if let Ok(mut txt) = cbctx.get_contents() {
                        txt.insert_str(0, "copy-text ");
                        sender.send_message(&OwnedMessage::Text(txt)).unwrap();
                    }
                } else if ctx.starts_with("file-list") {
                    let fss = get_files("files");
                    let mut res = fss.join("&");
                    res.insert_str(0, "file-list ");
                    sender.send_message(&OwnedMessage::Text(res)).unwrap();
                }
            }
            OwnedMessage::Ping(ping) => {
                let message = OwnedMessage::Pong(ping);
                sender.send_message(&message).unwrap();
            }
            OwnedMessage::Close(_) => {
                println!("Front transfer close !");
                return;
            }
            _ => {}
        }
    }
}