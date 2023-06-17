use std::{
    net::{Ipv4Addr, SocketAddrV4}, fs,
};

use tiny_http::{Header, Response, Server};

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

pub fn run(port: u16) {
    let host = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port);
    let server = Server::http(host).unwrap();
    let file_dir = "files";
    for request in server.incoming_requests() {
        let _url = request.url();
        if _url.starts_with("/files") {
            // 文件列表
            let fls = get_files(file_dir);
            let res = fls.join("&");
            let mut response = Response::from_string(res);
            response.add_header(Header {
                field: "Access-Control-Allow-Origin".parse().unwrap(),
                value: ascii::AsciiString::from_ascii("*").unwrap(),
            });
            request.respond(response).unwrap();
        } else if _url.starts_with("/download") {
            // 下载文件
            let ps = _url.split("/");
            let _file = ps.last().unwrap();
            let _file = percent_encoding::percent_decode_str(_file).decode_utf8().unwrap();
            eprintln!("download {}", _file);
            let file = fs::File::open(&format!("{}/{}", file_dir, _file)).unwrap();
            let mut response = Response::from_file(file);
            response.add_header(Header {
                field: "Access-Control-Allow-Origin".parse().unwrap(),
                value: ascii::AsciiString::from_ascii("*").unwrap(),
            });
            response.add_header(Header {
                field: "Content-Type".parse().unwrap(),
                value: ascii::AsciiString::from_ascii("application/octet-stream").unwrap(),
            });
            request.respond(response).unwrap();
        } else if _url.starts_with("/upload") {
            if request.method().as_str() == "OPTIONS" {
                let mut response = Response::from_string("");
                response.add_header(Header {
                    field: "Access-Control-Allow-Origin".parse().unwrap(),
                    value: ascii::AsciiString::from_ascii("*").unwrap(),
                });
                request.respond(response).unwrap();
                continue;
            }
            // 文件上传
            // let mut fname = String::new();
            // for h in request.headers() {
            //     eprintln!("{}", h.value);
            //     if h.field.as_str() == "Content-Disposition" {
            //         fname.push_str(h.value.as_str());
            //         break;
            //     }
            // }
            // eprintln!("{}", fname);
            // let filename = fname.split("filename=").nth(1).unwrap().trim_matches('"');
            // let mut file = File::create(format!("{}/{}", file_dir, filename)).unwrap();
            // let mut buffer = [0; 4096];
            // while let Some(size) = request.as_reader().read(&mut buffer).ok() {
            //     if size == 0 {
            //         break;
            //     }
            //     file.write_all(&buffer[..size]).unwrap();
            // }

            let mut response = Response::from_string("File uploaded successfully!");
            response.add_header(Header {
                field: "Access-Control-Allow-Origin".parse().unwrap(),
                value: ascii::AsciiString::from_ascii("*").unwrap(),
            });
            request.respond(response).unwrap();
        }
    }
}
