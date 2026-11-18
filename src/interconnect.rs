use crate::memory::Bios;

pub struct Interconnect {
    bios: Bios,
}

impl Interconnect {
    pub fn new(bios: Bios) -> Self {
        Interconnect {
            bios: bios,
        }
    }

    pub fn load_32(&self, addr: u32) -> u32 {
        if let Some(offset) = map::BIOS.contains(addr) {
            return  self.bios.load_32(offset);
        }

        panic!("unhandled_fetch32_at_address_{:08x}", addr);
    }
}

mod map {
    pub struct Range(u32, u32);

    impl Range {
        pub fn contains(self, addr: u32) -> Option<u32> {
            let Range(start, length) = self;

            if addr >= start && addr < start + length {
                Some(addr - start)
            } else {
                None
            }
        }
    }

    pub const BIOS: Range = Range(0xbfc00000, 512 * 1024);
}
