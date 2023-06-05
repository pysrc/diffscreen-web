pub struct Bitmap(u128, u128);

impl Bitmap {
    pub fn new() -> Self {
        Bitmap(0, 0)
    }
    pub fn push(&mut self, key: u8) -> bool {
        if key <= 127 {
            // 0-127
            let b = 1 << key;
            if self.1 & b == b {
                return false;
            }
            self.1 |= b;
        } else {
            // 128-255
            let b = 1 << (key - 128);
            if self.0 & b == b {
                return false;
            }
            self.0 |= b;
        }
        return true;
    }

    pub fn remove(&mut self, key: u8) {
        if key <= 127 {
            let b = !(1 << key);
            self.1 &= b;
        } else {
            let b = !(1 << (key - 128));
            self.0 &= b;
        }
    }
}
