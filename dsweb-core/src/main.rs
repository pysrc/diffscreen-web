#[macro_use]
extern crate rouille;
use clipboard::ClipboardContext;
use clipboard::ClipboardProvider;
use enigo::Enigo;
use enigo::KeyboardControllable;
use enigo::MouseControllable;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use rouille::websocket;
use rouille::Response;

mod convert;
mod imop;
mod key_mouse;
mod screen;

use std::fs;
use std::io::Write;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;
use std::{net::SocketAddrV4, str::FromStr};

use clap::{arg, command, Parser};
// use clipboard::{ClipboardContext, ClipboardProvider};
// use futures_util::{TryStreamExt, StreamExt, SinkExt};
use screen::Cap;
// use tokio::{io::AsyncWriteExt, sync::{mpsc, broadcast::Receiver}};
// use tokio_stream::wrappers::UnboundedReceiverStream;
// use warp::{Filter, multipart::FormData, Buf, ws::Message};

/// 判断文件夹是否存在，不存在创建
// fn fcheck(dir: &str) {
//     let mt = fs::metadata(dir);
//     if mt.is_err() {
//         fs::create_dir(dir).unwrap();
//     }
// }

fn get_files(dir: &str) -> Vec<String> {
    let mut res = Vec::<String>::new();
    let folder = fs::read_dir(dir).unwrap();
    for file in folder {
        let f = file.unwrap();
        if f.file_type().unwrap().is_file() {
            res.push(f.path().file_name().unwrap().to_string_lossy().to_string());
        }
    }
    return res;
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Bind
    #[arg(short, long, default_value_t = String::from("0.0.0.0:41290"))]
    bind: String,
    /// Static resource directory
    #[arg(short, long, default_value_t = String::from("public"))]
    webroot: String,
    /// File directory
    #[arg(short, long, default_value_t = String::from("files"))]
    files: String,
}

const M: usize = 4;
const N: usize = 4;

fn main() {
    let args = Args::parse();
    let bind = SocketAddrV4::from_str(&args.bind).unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();

    let (temp_sender, temp_receiver) = std::sync::mpsc::channel();

    let data_sender_map = Arc::new(Mutex::new(Vec::<Sender<Vec<u8>>>::new()));
    let wsmap = Arc::new(Mutex::new(
        Vec::<Arc<Mutex<Vec<websocket::Websocket>>>>::new(),
    ));
    for k in 0..M * M {
        let (s, r) = std::sync::mpsc::channel::<Vec<u8>>();
        if let Ok(mut m) = data_sender_map.lock() {
            m.push(s);
        }
        let wsa = Arc::new(Mutex::new(Vec::<websocket::Websocket>::new()));
        let wsac = wsa.clone();
        if let Ok(mut wp) = wsmap.lock() {
            wp.push(wsa);
        }

        // 第k个分组发送数据
        let temp_sender = temp_sender.clone();
        std::thread::spawn(move || {
            let mut buf = Vec::<u8>::with_capacity(1024 * 4);
            let mut e = DeflateEncoder::new(buf, Compression::default());
            e.write_all(&[0u8]).unwrap();
            buf = e.reset(Vec::new()).unwrap();
            while let Ok(temp) = r.recv() {
                // 压缩数据
                e.write_all(&temp).unwrap();
                unsafe {
                    buf.set_len(0);
                }
                buf = e.reset(buf).unwrap();
                // eprintln!("buf {}", buf.len());
                // 找出k分组的websocket，发送数据
                match wsac.lock() {
                    Ok(mut wsv) => {
                        // eprintln!("P {} {:p}", k, temp.as_ptr());
                        wsv.retain_mut(|wss| {
                            if let Err(_) = wss.send_binary(&buf) {
                                false
                            } else {
                                true
                            }
                        });
                    }
                    Err(_) => {}
                }
                // 将temp返回给截图线程
                temp_sender.send((k, temp)).unwrap();
            }
        });
    }
    std::thread::spawn(move || {
        let mut cap = Cap::new();
        let (w, h) = cap.wh();
        // 发送w, h, m, n
        let mut meta = vec![0u8; 6];
        meta[0] = (w >> 8) as u8;
        meta[1] = w as u8;
        meta[2] = (h >> 8) as u8;
        meta[3] = h as u8;
        meta[4] = M as u8;
        meta[5] = N as u8;
        sender.send(meta).unwrap();
        let (sw, sh, mut yuvs) = imop::split_meta_info(w, h, N, M);
        let (_, _, mut _yuvs) = imop::split_meta_info(w, h, N, M);
        let mut yuv = Vec::<u8>::new();
        loop {
            // 截图
            let bgra = cap.cap();
            convert::bgra_to_i420(w, h, bgra, &mut yuv);
            imop::split_i420_into_subimages(&yuv, &mut yuvs, w, h, N, M);
            // 找出变化的发送给前端
            let mut count = 0;
            for k in 0..M * N {
                // 对比Y分量即可
                let _new = yuvs.get(&k).unwrap();
                let _old = _yuvs.get(&k).unwrap();
                if _new[..sw * sh] == _old[..sw * sh] {
                    continue;
                }
                count += 1;
                let temp = yuvs.remove(&k).unwrap();
                // eprintln!("K {} {:p}", k, temp.as_ptr());
                // 发送到第k个分组
                if let Ok(sdr) = data_sender_map.lock() {
                    // todo 这里待修改
                    sdr[k].send(temp).unwrap();
                }
            }
            // 等待线程返回
            for _ in 0..count {
                let (k, temp) = temp_receiver.recv().unwrap();
                yuvs.insert(k, temp);
            }
            (_yuvs, yuvs) = (yuvs, _yuvs);
        }
    });
    let meta = receiver.recv().unwrap();
    rouille::start_server(bind, move |request| {
        router!(request,
            (GET) (/) => {
                Response::redirect_303("/index.html")
            },
            (GET) (/meta) => {
                Response::json(&meta)
            },
            (GET) (/ctrl) => {
                let (response, websocket) = try_or_400!(websocket::start(&request, Some("ctrl")));
                std::thread::spawn(move || {
                    let ws = websocket.recv().unwrap();
                    ctrlws(ws);
                });
                response
            },
            (GET) (/diffscreen/{k: usize}) => {
                let (response, websocket) = try_or_400!(websocket::start(&request, Some("diffscreen")));
                let wsmap = wsmap.clone();
                std::thread::spawn(move ||{
                    let rws = websocket.recv().unwrap();
                    if let Ok(wm) = wsmap.lock() {
                        if let Ok(mut wmv) = wm[k].lock() {
                            wmv.push(rws);
                        }
                    }
                });
                response
            },
            (GET) (/clipboard) => {
                // 粘贴板ws
                let (response, websocket) = try_or_400!(websocket::start(&request, Some("clipboard")));
                std::thread::spawn(move ||{
                    clipboardws(websocket.recv().unwrap());
                });
                response
            },
            (GET) (/list) => {
                // 获取文件列表
                let fls = get_files(&args.files);
                Response::json(&fls)
            },
            (GET) (/files/{filename: String}) => {
                // 文件下载
                eprintln!("download {}", filename);
                if let Some(request) = request.remove_prefix("/files") {
                    rouille::match_assets(&request, &args.files)
                } else {
                    rouille::Response::empty_404()
                }
            },
            (POST) (/upload) => {
                // 文件上传
                let data = try_or_400!(post_input!(request, {
                    file: rouille::input::post::BufferedFile,
                }));
                if let Some(filename) = data.file.filename {
                    let mut file = fs::File::create(format!("{}/{}", &args.files, filename)).unwrap();
                    file.write_all(&data.file.data).unwrap();
                }
                rouille::Response::html("Success!")
            },
            _ => {
                // 静态文件服务器
                rouille::match_assets(&request, &args.webroot)
            }
        )
    });
}

/**
 * 控制流ws
 */
fn ctrlws(mut ws: websocket::Websocket) {
    let mut enigo = Enigo::new();
    while let Some(msg) = ws.next() {
        if let websocket::Message::Binary(cmd) = msg {
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
    }
}

fn clipboardws(mut ws: websocket::Websocket) {
    let mut cbctx: ClipboardContext = ClipboardProvider::new().unwrap();
    while let Some(msg) = ws.next() {
        if let websocket::Message::Text(text) = msg {
            if text.starts_with("paste-text") {
                let text = text.replacen("paste-text ", "", 1);
                cbctx.set_contents(text).unwrap();
            } else if text.starts_with("copy-text") {
                if let Ok(mut txt) = cbctx.get_contents() {
                    txt.insert_str(0, "copy-text ");
                    ws.send_text(&txt).unwrap();
                }
            }
        }
    }
}
// struct MutiReceiver {
//     inner: Receiver<Message>
// }

// impl Clone for MutiReceiver {
//     fn clone(&self) -> Self {
//         Self { inner: self.inner.resubscribe() }
//     }
// }

// async fn async_server(srx: Receiver<Message>, meta: Message, args: Args) {
//     // 文件上传下载路径
//     static mut FILE_DIR: String = String::new();
//     // 静态web文件路径
//     static mut PUBLIC_RESOURCE: String = String::new();
//     unsafe {
//         FILE_DIR.push_str(&args.files);
//         PUBLIC_RESOURCE.push_str(&args.webroot);
//     }
//     unsafe {
//         fcheck(&FILE_DIR);
//         fcheck(&PUBLIC_RESOURCE);
//     }
//     // 静态请求
//     let public_route = warp::get().and(warp::fs::dir(unsafe { &PUBLIC_RESOURCE }));
//     // 文件列表
//     let file_list = warp::get().and(warp::path("list")).map(|| {
//         let fls = unsafe { get_files(&FILE_DIR) };
//         warp::reply::json(&fls)
//     });
//     // 文件下载
//     let file_download = warp::get()
//         .and(warp::path("files"))
//         .and(warp::fs::dir(unsafe {&FILE_DIR}));
//     // 文件上传
//     let upload = warp::multipart::form()
//     .and(warp::path("upload"))
//     .and_then(|form: FormData| async move {
//         let field_names: Vec<_> = form
//             .and_then(|mut field| async move {
//                 eprintln!("upload {}", field.filename().unwrap());
                // let mut file = tokio::fs::File::create(format!("{}/{}", unsafe{&FILE_DIR}, field.filename().unwrap())).await.unwrap();
                // while let Some(content) = field.data().await {
                //     let content = content.unwrap();
                //     let chunk: &[u8] = content.chunk();
                //     file.write_all(chunk).await.unwrap();
                // }
//                 Ok(())
//             })
//             .try_collect()
//             .await
//             .unwrap();

//         Ok::<_, warp::Rejection>(format!("{:?}", field_names))
//     });
//     // 处理websocket
//     let mr = MutiReceiver{
//         inner: srx
//     };
//     let mr = warp::any().map(move || mr.clone());
//     let meta = warp::any().map(move || meta.clone());
//     let diffscreen_ws = warp::path("diffscreen")
//     .and(warp::ws())
//     .and(mr)
//     .and(meta)
//     .map(|ws: warp::ws::Ws, mr: MutiReceiver, meta: Message| {
//         ws.on_upgrade(|websocket| async move {
//             eprintln!("start");
//             let (mut user_ws_tx, mut user_ws_rx) = websocket.split();
//             user_ws_tx.send(meta).await.unwrap();
//             tokio::task::spawn(async move {
//                 let mut mr = mr.clone();
//                 while let Ok(message) = mr.inner.recv().await {
//                     if let Err(_) = user_ws_tx.send(message).await {
//                         eprintln!("break task 1");
//                         break;
//                     }
//                 }
//             });
//             // 控制信息处理
//             let (tx, rx) = mpsc::unbounded_channel::<Message>();
//             let rx = UnboundedReceiverStream::new(rx);
//             tokio::task::spawn(async move {
//                 while let Some(msg) = user_ws_rx.next().await {
//                     match msg {
//                         Ok(message) => {
//                             tx.send(message).unwrap();
//                         }
//                         Err(e) => {
//                             eprintln!("err {}", e);
//                         }
//                     }

//                 }
//                 tx.send(Message::close()).unwrap();
//                 eprintln!("break task 2");
//             });
//             tokio::task::spawn(async move {
//                 ctrl_event::ctrl(rx).await;
//                 eprintln!("break task 3");
//             });
//         })
//     });

//     let clipboard_ws = warp::path("diffscreen-cb")
//     .and(warp::ws())
//     .map(|ws: warp::ws::Ws| {
//         ws.on_upgrade(|websocket| async move {
//             let (mut user_ws_tx, mut user_ws_rx) = websocket.split();
//             let (tx, rx) = mpsc::unbounded_channel::<Message>();
//             tokio::task::spawn(async move {
//                 let mut cbctx: ClipboardContext = ClipboardProvider::new().unwrap();
//                 while let Some(msg) = user_ws_rx.next().await {
//                     match msg {
//                         Ok(message) => {
//                             if message.is_text() {
//                                 let ctx = message.to_str().unwrap();
                                // if ctx.starts_with("paste-text") {
                                //     let ctx = ctx.replacen("paste-text ", "", 1);
                                //     cbctx.set_contents(ctx).unwrap();
                                // } else if ctx.starts_with("copy-text") {
                                //     if let Ok(mut txt) = cbctx.get_contents() {
                                //         txt.insert_str(0, "copy-text ");
                                //         let txt = Message::text(txt);
                                //         tx.send(txt).unwrap();
                                //     }
                                // }
//                             }
//                         }
//                         Err(e) => {
//                             eprintln!("err {}", e);
//                         }
//                     }
//                 }
//             });
//             let mut rx = UnboundedReceiverStream::new(rx);
//             tokio::task::spawn(async move {
//                 while let Some(msg) = rx.next().await {
//                     if msg.is_close() {
//                         break;
//                     }
//                     user_ws_tx.send(msg).await.unwrap();
//                 }
//             });
//         })
//     });
//     let bind = SocketAddrV4::from_str(&args.bind).unwrap();
//     warp::serve(public_route.or(file_list).or(file_download).or(upload).or(diffscreen_ws).or(clipboard_ws))
//         .run(bind)
//         .await;
// }
