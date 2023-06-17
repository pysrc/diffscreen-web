mod key_mouse;
mod screen;
mod server;
mod config;
mod imop;
mod http_server;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    // defalut port
    let mut port = 41290;
    let mut http_port = 41291;
    if args.len() >= 2 {
        port = args[1].parse::<u16>().unwrap();
    }
    if args.len() >= 3 {
        http_port = args[2].parse::<u16>().unwrap();
    }
    std::thread::spawn(move ||{
        http_server::run(http_port);
    });
    // run forever
    server::run(port);
}
