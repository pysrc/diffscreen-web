
/// yuv图像分裂成m行n列
pub fn split_meta_info(
    width: usize,
    height: usize,
    n: usize,
    m: usize,
) -> (usize, usize, Vec<Option<Vec<u8>>>) {
    let mut subimages = Vec::with_capacity(n * m);
    let subimage_width = width / n;
    let subimage_height = height / m;
    for _ in 0..n * m {
        subimages.push(Some(vec![0u8; subimage_width * subimage_height * 3 / 2]));
    }
    (subimage_width, subimage_height, subimages)
}

/// 将i420图像切割成m行n列的子图像列表
pub fn split_i420_into_subimages(
    i420_data: &Vec<u8>,
    subimages: &mut Vec<Option<Vec<u8>>>,
    width: usize,
    height: usize,
    n: usize,
    m: usize,
) {
    let plane_size = width * height;

    // Calculate the width and height of each subimage
    let subimage_width = width / n;
    let subimage_height = height / m;
    let subplane_size = subimage_width * subimage_height;

    let sw2 = subimage_width / 2;

    let mut k = 0;
    for j in 0..m {
        for i in 0..n {
            let img = subimages.get_mut(k).unwrap();
            if let Some(img) = img {
                // Calculate the starting index for each plane (Y, U, V)
                let y_start = (j * subimage_height) * width + (i * subimage_width);
                let u_start = plane_size
                    + ((j * subimage_height) / 2) * (width / 2)
                    + ((i * subimage_width) / 2); // U plane
                let v_start = plane_size
                    + (plane_size / 4)
                    + ((j * subimage_height) / 2) * (width / 2)
                    + ((i * subimage_width) / 2); // V plane

                for row in 0..subimage_height {
                    // Extract Y plane data
                    let y_offset = y_start + row * width;
                    // Y分量
                    let _ = &mut img[..subplane_size]
                        [row * subimage_width..row * subimage_width + subimage_width]
                        .copy_from_slice(&i420_data[y_offset..y_offset + subimage_width]);

                    if row % 2 == 0 {
                        // Extract U and V plane data (subsampling)
                        let uv_row = row / 2;
                        let u_offset = u_start + uv_row * (width / 2);
                        let v_offset = v_start + uv_row * (width / 2);
                        // U分量
                        let _ = &mut img[subplane_size..subplane_size + subplane_size / 4]
                            [uv_row * sw2..uv_row * sw2 + sw2]
                            .copy_from_slice(&i420_data[u_offset..u_offset + sw2]);
                        // V分量
                        let _ = &mut img[subplane_size + subplane_size / 4..]
                            [uv_row * sw2..uv_row * sw2 + sw2]
                            .clone_from_slice(&i420_data[v_offset..v_offset + sw2]);
                    }
                }
            }
            k += 1;
        }
    }
}
