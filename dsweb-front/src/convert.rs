fn clamp(x: i32) -> u8 {
    x.min(255).max(0) as u8
}

pub fn i420_to_rgba(width: usize, height: usize, sy: &[u8], su: &[u8], sv: &[u8], dest: &mut [u8]) {
    let uvw = width >> 1;
    for i in 0..height {
        let sw = i * width;
        let t = (i >> 1) * uvw;
        for j in 0..width {
            let rgbstart = sw + j;
            let uvi = t + (j >> 1);

            let y = sy[rgbstart] as i32;
            let u = su[uvi] as i32 - 128;
            let v = sv[uvi] as i32 - 128;

            let rgba = rgbstart * 4;

            dest[rgba] = clamp(y + (v * 359 >> 8));
            dest[rgba + 1] = clamp(y - (u * 88 >> 8) - (v * 182 >> 8));
            dest[rgba + 2] = clamp(y + (u * 453 >> 8));
            dest[rgba + 3] = 0xff;

        }
    }

}