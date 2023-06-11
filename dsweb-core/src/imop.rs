use crate::config;

/// 将bgra数组转换为rgb块
pub fn sub_areas_bgra(bgra: &[u8], recs: &mut Vec<Vec<u8>>, w: usize, h: usize, sw: usize, sh: usize, offset: usize) {
    let mut i = 0;
    for y in (0..h).step_by(sh) {
        for x in (0..w).step_by(sw) {
            let rec = &mut recs[i];
            let rec = &mut rec[offset..];
            for j in 0..sh {
                let py = y + j;
                for k in 0..sw {
                    let px = x + k;
                    if px < w && py < h {
                        let index = (py*w + px) * 4;
                        let r = bgra[index + 2];
                        let g = bgra[index + 1];
                        let b = bgra[index];
                        let rec_index = (j*sw + k) * 3;
                        rec[rec_index] = r & config::BIT_MASK;
                        rec[rec_index+1] = g & config::BIT_MASK;
                        rec[rec_index+2] = b & config::BIT_MASK;
                    }
                }
            }
            i += 1;
        }
    }
}