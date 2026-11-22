use std::error::Error;
use std::fs::File;
use std::io::Read;

use crate::interconnect::Addressable;


const BIOS_SIZE: usize = 512 * 1024;

pub struct Bios {
    data: Vec<u8>
}

impl Bios {

    pub fn new(path: &str) -> Result<Self, Box<dyn Error>> {
        let mut data = vec![0u8; BIOS_SIZE];
        let mut file = File::open(path)?;
        
        file.read_exact(&mut data)?;
        Ok(Bios { data })
    }

    pub fn load<T: Addressable>(&self, offset: u32) -> T {
        let offset = offset as usize;

        let mut v = 0;

        for i in 0..T::width() as usize {
            v |= (self.data[offset + i] as u32) << (i * 8)
        }

        Addressable::from_u32(v)
    }
}