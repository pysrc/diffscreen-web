mod screen;
mod imop;
mod config;
mod screen_stream;
mod ctrl_event;
mod key_mouse;

use std::{fs, net::SocketAddrV4, str::FromStr};

use clap::{command, Parser, arg};
use clipboard::{ClipboardContext, ClipboardProvider};
use futures_util::{TryStreamExt, StreamExt, SinkExt};
use screen::Cap;
use tokio::{io::AsyncWriteExt, sync::{mpsc, broadcast::Receiver}};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::{Filter, multipart::FormData, Buf, ws::Message};

/// 判断文件夹是否存在，不存在创建
fn fcheck(dir: &str) {
    let mt = fs::metadata(dir);
    if mt.is_err() {
        fs::create_dir(dir).unwrap();
    }
}

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

fn main() {
    let args = Args::parse();
    let cap = Cap::new(config::SW, config::SH, 2);
    let (w, h, sw, sh, _) = cap.size_info();
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
    let meta = Message::binary(meta);
    let (tx, rx) = tokio::sync::broadcast::channel::<Message>(10);
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            async_server(rx, meta, args).await;
        });
    });
    screen_stream::screen_stream(cap, tx);
}

struct MutiReceiver {
    inner: Receiver<Message>
}

impl Clone for MutiReceiver {
    fn clone(&self) -> Self {
        Self { inner: self.inner.resubscribe() }
    }
}

async fn async_server(srx: Receiver<Message>, meta: Message, args: Args) {
    // 文件上传下载路径
    static mut FILE_DIR: String = String::new();
    // 静态web文件路径
    static mut PUBLIC_RESOURCE: String = String::new();
    unsafe {
        FILE_DIR.push_str(&args.files);
        PUBLIC_RESOURCE.push_str(&args.webroot);
    }
    unsafe {
        fcheck(&FILE_DIR);
        fcheck(&PUBLIC_RESOURCE);
    }
    // 静态请求
    let public_route = warp::get().and(warp::fs::dir(unsafe { &PUBLIC_RESOURCE }));
    // 文件列表
    let file_list = warp::get().and(warp::path("list")).map(|| {
        let fls = unsafe { get_files(&FILE_DIR) };
        warp::reply::json(&fls)
    });
    // 文件下载
    let file_download = warp::get()
        .and(warp::path("files"))
        .and(warp::fs::dir(unsafe {&FILE_DIR}));
    // 文件上传
    let upload = warp::multipart::form()
    .and(warp::path("upload"))
    .and_then(|form: FormData| async move {
        let field_names: Vec<_> = form
            .and_then(|mut field| async move {
                eprintln!("upload {}", field.filename().unwrap());
                let mut file = tokio::fs::File::create(format!("{}/{}", unsafe{&FILE_DIR}, field.filename().unwrap())).await.unwrap();
                while let Some(content) = field.data().await {
                    let content = content.unwrap();
                    let chunk: &[u8] = content.chunk();
                    file.write_all(chunk).await.unwrap();
                }
                Ok(())
            })
            .try_collect()
            .await
            .unwrap();

        Ok::<_, warp::Rejection>(format!("{:?}", field_names))
    });
    // 处理websocket
    let mr = MutiReceiver{
        inner: srx
    };
    let mr = warp::any().map(move || mr.clone());
    let meta = warp::any().map(move || meta.clone());
    let diffscreen_ws = warp::path("diffscreen")
    .and(warp::ws())
    .and(mr)
    .and(meta)
    .map(|ws: warp::ws::Ws, mr: MutiReceiver, meta: Message| {
        ws.on_upgrade(|websocket| async move {
            eprintln!("start");
            let (mut user_ws_tx, mut user_ws_rx) = websocket.split();
            user_ws_tx.send(meta).await.unwrap();
            tokio::task::spawn(async move {
                let mut mr = mr.clone();
                while let Ok(message) = mr.inner.recv().await {
                    if let Err(_) = user_ws_tx.send(message).await {
                        eprintln!("break task 1");
                        break;
                    }
                }
            });
            // 控制信息处理
            let (tx, rx) = mpsc::unbounded_channel::<Message>();
            let rx = UnboundedReceiverStream::new(rx);
            tokio::task::spawn(async move {
                while let Some(msg) = user_ws_rx.next().await {
                    match msg {
                        Ok(message) => {
                            tx.send(message).unwrap();
                        }
                        Err(e) => {
                            eprintln!("err {}", e);
                        }
                    }
                    
                }
                tx.send(Message::close()).unwrap();
                eprintln!("break task 2");
            });
            tokio::task::spawn(async move {
                ctrl_event::ctrl(rx).await;
                eprintln!("break task 3");
            });
        })
    });

    let clipboard_ws = warp::path("diffscreen-cb")
    .and(warp::ws())
    .map(|ws: warp::ws::Ws| {
        ws.on_upgrade(|websocket| async move {
            let (mut user_ws_tx, mut user_ws_rx) = websocket.split();
            let (tx, rx) = mpsc::unbounded_channel::<Message>();
            tokio::task::spawn(async move {
                let mut cbctx: ClipboardContext = ClipboardProvider::new().unwrap();
                while let Some(msg) = user_ws_rx.next().await {
                    match msg {
                        Ok(message) => {
                            if message.is_text() {
                                let ctx = message.to_str().unwrap();
                                if ctx.starts_with("paste-text") {
                                    let ctx = ctx.replacen("paste-text ", "", 1);
                                    cbctx.set_contents(ctx).unwrap();
                                } else if ctx.starts_with("copy-text") {
                                    if let Ok(mut txt) = cbctx.get_contents() {
                                        txt.insert_str(0, "copy-text ");
                                        let txt = Message::text(txt);
                                        tx.send(txt).unwrap();
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("err {}", e);
                        }
                    }
                }
            });
            let mut rx = UnboundedReceiverStream::new(rx);
            tokio::task::spawn(async move {
                while let Some(msg) = rx.next().await {
                    if msg.is_close() {
                        break;
                    }
                    user_ws_tx.send(msg).await.unwrap();
                }
            });
        })
    });
    let bind = SocketAddrV4::from_str(&args.bind).unwrap();
    warp::serve(public_route.or(file_list).or(file_download).or(upload).or(diffscreen_ws).or(clipboard_ws))
        .run(bind)
        .await;
}
