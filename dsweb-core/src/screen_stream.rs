

use std::io::Write;

use flate2::{write::DeflateEncoder, Compression};

use tokio::sync::broadcast::Sender;
use warp::ws::Message;

use crate::screen::Cap;

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

pub fn screen_stream(mut cap: Cap, stream: Sender<Message>) {
    let (w, h, sw, sh, _) = cap.size_info();
    let (mut a, srw, srh) = get_rgb_block(w, h, sw, sh, 2);
    let block_len = srw * srh;
    let (mut b, _, _) = get_rgb_block(w, h, sw, sh, 2);
    let mut k = 0usize;
    loop {
        cap.cap(&mut b);
        // 对比a
        let mut sendbuf = Vec::<u8>::with_capacity(1024 * 4);
        let mut e = DeflateEncoder::new(sendbuf, Compression::default());
        // 每100帧重新同步
        if k > 100 {
            k = 0;
            for i in 0..block_len {
                e.write_all(&b[i]).unwrap();
            }
        } else {
            for i in 0..block_len {
                let (a, b) = (&a[i], &b[i]);
                if a != b {
                    e.write_all(&b).unwrap();
                }
            }
        }
        sendbuf = e.finish().unwrap();
        if sendbuf.len() > 0 {
            // 这里怎么避免内存频繁申请释放？
            let msg = Message::binary(sendbuf);
            stream.send(msg).unwrap();
        }
        (a, b) = (b, a);
        k += 1;
    }
}
