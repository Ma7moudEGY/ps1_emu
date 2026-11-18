use std::error::Error;
use std::fs::File;
use std::io::Read;


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

    pub fn load_32(&self, offset: u32) -> u32 {
        let offset = offset as usize;

        let b0 = self.data[offset + 0] as u32;
        let b1 = self.data[offset + 1] as u32;
        let b2 = self.data[offset + 2] as u32;
        let b3 = self.data[offset + 3] as u32;

        b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    }
}