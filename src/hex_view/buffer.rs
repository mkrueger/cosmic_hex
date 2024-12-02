pub struct DataBuffer {
    pub data: Vec<u8>,
}

impl DataBuffer {
    pub fn set_byte(&mut self, offset: usize, value: u8) {
        self.data[offset] = value;
    }

    pub fn get_byte(&self, offset: usize) -> u8 {
        self.data[offset]
    }

    pub fn get_u32(&self, offset: usize) -> u32 {
        let mut result = 0;
        for i in 0..4 {
            result |= (self.get_byte(offset + i) as u32) << (i * 8);
        }
        result
    }

    pub fn get_i32(&self, offset: usize) -> i32 {
        let mut result = 0;
        for i in 0..4 {
            result |= (self.get_byte(offset + i) as i32) << (i * 8);
        }
        result
    }

    pub(crate) fn len(&self) -> usize {
        self.data.len()
    }
}
