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

    pub fn load_8(&self, addr: u32) -> u8 {
        let abs_addr = map::mask_region(addr);

        if let Some(offset) = map::BIOS.contains(abs_addr) {
            return self.bios.load_8(offset);
        }

        if let Some(_) = map::EXPANSION_1.contains(abs_addr) {
            return 0xff;
        }

        if let Some(offset) = map::RAM.contains(abs_addr) {
            return self.ram.load_8(offset);
        }

        todo!("unhandled_load_8_at_address_{:08x}", addr);
    }

    pub fn load_32(&self, addr: u32) -> u32 {
        let addr = map::mask_region(addr);

        if addr % 4 != 0 {
            todo!("unaligned_load_32_address_{:08x}", addr);
        }

        if let Some(offset) = map::BIOS.contains(addr) {
            return self.bios.load_32(offset);
        }

        if let Some(offset) = map::RAM.contains(addr) {
            return self.ram.load_32(offset);
        }

        if let Some(offset) = map::IRQ_CONTROL.contains(addr) {
            println!("IRQ_CONTROL_read_{:x}", offset);
            return 0;
        }

        if let Some(_) = map::DMA.contains(addr) {
            println!("DMA_read_{:08x}", addr);
            return 0;
        }

        todo!("unhandled_load_32_at_address_{:08x}", addr);
    }

    pub fn store_8(&mut self, addr: u32, val: u8) {
        let abs_addr = map::mask_region(addr);

        if let Some(offset) = map::EXPANSION_2.contains(abs_addr) {
            println!("unhandled_write_to_EXPANSION_2_register_{:x}", offset);
            return;
        }

        if let Some(offset) = map::RAM.contains(abs_addr) {
            self.ram.store_8(offset, val);
            return;
        }

        todo!("unhandled_store_8_address_{:08x}", addr);
    }

    pub fn store_16(&mut self, addr: u32, val: u16) {
        if addr % 2 != 0 {
            todo!("unaligned_store_16_address_{:08x}", addr);
        }

        let abs_addr = map::mask_region(addr);

        if let Some(offset) = map::SPU.contains(abs_addr) {
            println!("unhandled_write_to_SPU_register_{:x}", offset);
            return;
        }

        if let Some(offset) = map::TIMERS.contains(abs_addr) {
            println!("uhandled_write_to_timer_register_{:x}", offset);
            return;
        }

        if let Some(offset) = map::RAM.contains(abs_addr) {
            return self.ram.store_16(offset, val);
        }

        todo!("unhandled_store_16_address_{:08x}", addr);
    }

    pub fn store_32(&mut self, addr: u32, val: u32) {
        let addr = map::mask_region(addr);

        if addr % 4 != 0 {
            todo!("unaligned_store_32_address_{:08x}", addr);
        }

        if let Some(offset) = map::SYS_CONTROL.contains(addr) {
            match offset {
                0 => {
                    if val != 0x1f000000 {
                        todo!("bad_expansion_1_base_address:_0x{:08x}", val);
                    }
                }

                4 => {
                    if val != 0x1f802000 {
                        todo!("bad_expansion_2_base_address:_0x{:08x}", val);
                    }
                }

                _ => println!("unhandled_write_to_SYS_CONTROL_register"),
            }
            return;
        }

        if let Some(offset) = map::RAM_SIZE.contains(addr) {
            println!("ignoring_write_to_RAM_SIZE_register_{:x}", offset);
            return;
        }

        if let Some(offset) = map::RAM.contains(addr) {
            self.ram.store_32(offset, val);
            return;
        }

        if let Some(x) = map::SYS_CONTROL.contains(addr) {
            println!("writing_{:08x}_to_sys_control", x);
            return;
        }

        if let Some(x) = map::CACHE_CONTROL.contains(addr) {
            println!("writing_{:08x}_to_cache_control", x);
            return;
        }

        if let Some(offset) = map::IRQ_CONTROL.contains(addr) {
            println!("IRQ_CONTROL:_{:x}_<-_{:08x}", offset, val);
            return;
        }

        if let Some(_) = map::DMA.contains(addr) {
            println!("DMA_write:_{:08x}_{:08x}", addr, val);
            return;
        }

        todo!("unhandled_store_32_into_address_{:08x}", addr);
    }
}

mod map {
    pub struct Range(u32, u32);
    pub const REGION_MASK: [u32; 8] = [
        0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0x7fffffff, 0x1fffffff, 0xffffffff,
        0xffffffff,
    ];

    pub fn mask_region(addr: u32) -> u32 {
        let index = (addr >> 29) as usize;

        addr & REGION_MASK[index]
    }

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

    pub const RAM: Range = Range(0x00000000, 2 * 1024 * 1024);

    pub const BIOS: Range = Range(0x1fc00000, 512 * 1024);

    pub const SYS_CONTROL: Range = Range(0x1f801000, 36);

    pub const RAM_SIZE: Range = Range(0x1f801060, 4);

    pub const SPU: Range = Range(0x1f801c00, 640);

    pub const CACHE_CONTROL: Range = Range(0xfffe0130, 4);

    pub const EXPANSION_1: Range = Range(0x1f000000, 8 * 1024 * 1024);

    pub const EXPANSION_2: Range = Range(0x1f802000, 66);

    pub const IRQ_CONTROL: Range = Range(0x1f801070, 8);

    pub const TIMERS: Range = Range(0x1f801100, 0x30);

    pub const DMA: Range = Range(0x1f801080, 0x80);
}
