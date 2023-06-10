use scrap::Capturer;
use scrap::Display;
use std::io::ErrorKind::WouldBlock;
use std::time::Duration;
use crate::imop;

/**
 * 截屏
 */
pub struct Cap {
    w: usize,
    h: usize,
    sw: usize, 
    sh: usize, 
    offset: usize,
    capturer: Option<Capturer>,
    sleep: Duration,
}
impl Cap {
    pub fn new(sw: usize, sh: usize, offset: usize) -> Cap {
        let display = Display::primary().unwrap();
        let capturer = Capturer::new(display).unwrap();
        let (w, h) = (capturer.width(), capturer.height());
        Cap {
            w,
            h,
            sw,
            sh,
            offset,
            capturer: Some(capturer),
            sleep: Duration::new(1, 0) / 60,
        }
    }
    fn reload(&mut self) {
        println!("Reload capturer");
        drop(self.capturer.take());
        let display = match Display::primary() {
            Ok(display) => display,
            Err(_) => {
                return;
            }
        };

        let capturer = match Capturer::new(display) {
            Ok(capturer) => capturer,
            Err(_) => return,
        };
        self.capturer = Some(capturer);
    }
    #[inline]
    pub fn size_info(&self) -> (usize, usize, usize, usize, usize) {
        return (self.w, self.h, self.sw, self.sh, self.offset);
    }
    #[inline]
    pub fn cap(&mut self, cap_buf: &mut Vec<Vec<u8>>) {
        loop {
            match &mut self.capturer {
                Some(capturer) => {
                    // Wait until there's a frame.
                    let cp = capturer.frame();
                    let buffer = match cp {
                        Ok(buffer) => buffer,
                        Err(error) => {
                            std::thread::sleep(self.sleep);
                            if error.kind() == WouldBlock {
                                // Keep spinning.
                                continue;
                            } else {
                                std::thread::sleep(std::time::Duration::from_millis(200));
                                self.reload();
                                continue;
                            }
                        }
                    };
                    // 转换成rgb图像数组
                    imop::sub_areas_bgra(&buffer, cap_buf, self.w, self.h, self.sw, self.sh, self.offset);
                    break;
                }
                None => {
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    self.reload();
                    continue;
                }
            };
        }
    }
}
