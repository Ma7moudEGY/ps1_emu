use crate::interconnect::Addressable;

pub struct Ram {
    data: Vec<u8>
}

impl Ram {
    pub fn new() -> Self {
        let data = vec![0xca; 2 * 1024 * 1024];
        Ram {
            data: data
        }
    }

    pub fn load<T: Addressable>(&self, offset: u32) -> T {
        let offset = offset as usize;

        let mut v = 0;

        for i in 0..T::width() as usize {
            v |= (self.data[offset + i] as u32) << (i * 8)
        }

        Addressable::from_u32(v)
    }

    pub fn store<T: Addressable>(&mut self, offset: u32, val: T) {
        let offset = offset as usize;

        let val = val.as_u32();

        for i in 0..T::width() as usize {
            self.data[offset + i] = (val >> (i * 8)) as u8;
        }
    }
}