use crate::memory::{Bios, Ram};

pub struct Interconnect {
    bios: Bios,
    ram: Ram,
}

impl Interconnect {
    pub fn new(bios: Bios, ram: Ram) -> Self {
        Interconnect {
            bios: bios,
            ram: ram,
        }
    }

    pub fn load_32(&self, addr: u32) -> u32 {
        if addr %4 != 0 {
            panic!("unaligned_load_32_address_{:08x}", addr);
        }

        if let Some(offset) = map::BIOS.contains(addr) {
            return self.bios.load_32(offset);
        }

        if let Some(offset) = map::RAM.contains(addr) {
            return self.ram.load_32(offset);
        }

        panic!("unhandled_fetch32_at_address_{:08x}", addr);
    }

    pub fn store_32(&mut self, addr: u32, val: u32) {
        if addr %4 != 0 {
            panic!("unaligned_store_32_address_{:08x}", addr);
        }

        if let Some(offset) = map::MEM_CONTROL.contains(addr) {
            match offset {
                0 =>
                    if val != 0x1f000000 {
                        panic!("bad_expansion_1_base_address:_0x{:08x}", val);
                    },

                4 =>
                    if val != 0x1f802000 {
                        panic!("bad_expansion_2_base_address:_0x{:08x}", val);
                    },

                _ =>
                    println!("unhandled_write_to_MEM_CONTROL_register"),
            }
            return;
        }

        if let Some(x) = map::RAM_SIZE.contains(addr) {
            println!("writing_{:08x}_to_ram", x);
            return;
        }

        if let Some(offset) = map::RAM.contains(addr) {
            self.ram.store_32(offset, val);
            return;
        }

        if let Some(x) = map::CACHE_CONTROL.contains(addr) {
            println!("writing_{:08x}_to_cache_control", x);
            return;
        }

        panic!("unhandled_store_32_into_address_{:08x}", addr);
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
    pub const MEM_CONTROL: Range = Range(0x1f801000, 36);
    pub const RAM_SIZE: Range = Range(0x1f801060, 4);
    pub const CACHE_CONTROL: Range = Range(0xfffe0130, 4);
    pub const RAM: Range = Range(0xa0000000, 2 * 1024 * 1024);
}
