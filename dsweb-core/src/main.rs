mod key_mouse;
mod screen;
mod server;
mod config;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    // defalut port
    let mut port = 41290;
    if args.len() >= 3 {
        port = args[2].parse::<u16>().unwrap();
    }

    // run forever
    server::run(port);
}
