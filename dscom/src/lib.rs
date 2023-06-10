
// key事件 start
pub const KEY_UP: u8 = 1;
pub const KEY_DOWN: u8 = 2;
pub const MOUSE_KEY_UP: u8 = 3;
pub const MOUSE_KEY_DOWN: u8 = 4;
pub const MOUSE_WHEEL_UP: u8 = 5;
pub const MOUSE_WHEEL_DOWN: u8 = 6;
pub const MOVE: u8 = 7;
// key事件 end

/// 初始化rgb块
pub fn get_rgb_block(w: usize, h: usize, sw: usize, sh: usize, offset: usize) -> (Vec<Vec<u8>>, usize, usize) {
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
