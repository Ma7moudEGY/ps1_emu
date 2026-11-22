use crate::{
    dma::{Direction, Dma, Port, Step, Sync}, gpu::Gpu, memory::{Bios, Ram},
};

pub struct Interconnect {
    bios: Bios,
    ram: Ram,
    dma: Dma,
    gpu: Gpu
}

impl Interconnect {
    pub fn new(bios: Bios, ram: Ram, dma: Dma, gpu: Gpu) -> Self {
        Interconnect {
            bios: bios,
            ram: ram,
            dma: dma,
            gpu: gpu,
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

    pub fn load_16(&self, addr: u32) -> u16 {
        let abs_addr = map::mask_region(addr);

        if let Some(_) = map::SPU.contains(abs_addr) {
            println!("unhandled_read_from_SPU_register_{:08x}", abs_addr);
            return 0;
        }

        if let Some(offset) = map::RAM.contains(abs_addr) {
            return self.ram.load_16(offset);
        }

        if let Some(offset) = map::IRQ_CONTROL.contains(abs_addr) {
            println!("IRQ_CONTROL_read_{:x}", offset);
            return 0;
        }

        panic!("unhandled_load_16_at_address_{:08x}", addr);
    }

    pub fn load_32(&self, addr: u32) -> u32 {
        let abs_addr = map::mask_region(addr);

        if addr % 4 != 0 {
            panic!("unaligned_load_32_address_{:08x}", addr);
        }

        if let Some(offset) = map::BIOS.contains(abs_addr) {
            return self.bios.load_32(offset);
        }

        if let Some(offset) = map::RAM.contains(abs_addr) {
            return self.ram.load_32(offset);
        }

        if let Some(offset) = map::IRQ_CONTROL.contains(abs_addr) {
            println!("IRQ_CONTROL_read_{:x}", offset);
            return 0;
        }

        if let Some(offset) = map::DMA.contains(abs_addr) {
            return self.dma_reg(offset);
        }

        if let Some(offset) = map::GPU.contains(abs_addr) {
            println!("GPU_read_{}", offset);

            return match offset {
                4 => self.gpu.status(),
                _ => 0,
            };
        }

        if let Some(offset) = map::TIMERS.contains(abs_addr) {
            println!("unhandled_read_from_timer_register_{:x}", offset);
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

        if let Some(offset) = map::IRQ_CONTROL.contains(abs_addr) {
            println!("IRQ_CONTROL:_{:x}_<-_{:08x}", offset, val);
            return;
        }

        todo!("unhandled_store_16_address_{:08x}", addr);
    }

    pub fn store_32(&mut self, addr: u32, val: u32) {
        let abs_addr = map::mask_region(addr);

        if addr % 4 != 0 {
            panic!("unaligned_store_32_address_{:08x}", addr);
        }

        if let Some(offset) = map::SYS_CONTROL.contains(abs_addr) {
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

        if let Some(offset) = map::RAM_SIZE.contains(abs_addr) {
            println!("ignoring_write_to_RAM_SIZE_register_{:x}", offset);
            return;
        }

        if let Some(offset) = map::RAM.contains(abs_addr) {
            self.ram.store_32(offset, val);
            return;
        }

        if let Some(x) = map::SYS_CONTROL.contains(abs_addr) {
            println!("writing_{:08x}_to_sys_control", x);
            return;
        }

        if let Some(x) = map::CACHE_CONTROL.contains(abs_addr) {
            println!("writing_{:08x}_to_cache_control", x);
            return;
        }

        if let Some(offset) = map::IRQ_CONTROL.contains(abs_addr) {
            println!("IRQ_CONTROL:_{:x}_<-_{:08x}", offset, val);
            return;
        }

        if let Some(offset) = map::DMA.contains(abs_addr) {
            return self.set_dma_reg(offset, val);
        }

        if let Some(offset) = map::GPU.contains(abs_addr) {
            match offset {
                0 => self.gpu.gp0(val),
                4 => self.gpu.gp1(val),
                _ => todo!("GPU_write_{}:{:08x}", offset, val),
            }
            return;
        }

        if let Some(offset) = map::TIMERS.contains(abs_addr) {
            println!("uhandled_write_to_timer_register_{:x}", offset);
            return;
        }

        todo!("unhandled_store_32_into_address_{:08x}:_{:08x}", addr, val);
    }

    fn dma_reg(&self, offset: u32) -> u32 {
        let major = (offset & 0x70) >> 4;
        let minor = offset & 0xf;

        match major {
            0..=6 => {
                let channel = self.dma.channel(Port::from_index(major));

                match minor {
                    0 => channel.base(),
                    4 => channel.block_control(),
                    8 => channel.control(),
                    _ => todo!("uhandled_DMA_read_at_{:x}", offset),
                }
            }

            7 => match minor {
                0 => self.dma.control(),
                4 => self.dma.interrupt(),
                _ => todo!("uhandled_DMA_read_at_{:x}", offset),
            },

            _ => todo!("uhandled_DMA_read_at_{:x}", offset),
        }
    }

    fn set_dma_reg(&mut self, offset: u32, val: u32) {
        let major = (offset & 0x70) >> 4;
        let minor = offset & 0xf;

        let active_port = match major {
            0..=6 => {
                let port = Port::from_index(major);
                let channel = self.dma.channel_mut(port);

                match minor {
                    0 => channel.set_base(val),
                    4 => channel.set_block_control(val),
                    8 => channel.set_control(val),
                    _ => todo!("uhandled_DMA_write_at_{:x}:_{:08x}", offset, val),
                }

                if channel.active() { 
                    Some(port)
                } else { 
                    None }
            }

            7 => {
                match minor {
                    0 => self.dma.set_control(val),
                    4 => self.dma.set_interrupt(val),
                    _ => todo!("uhandled_DMA_write_at_{:x}:_{:08x}", offset, val),
                }

                None
            }
            _ => todo!("uhandled_DMA_write_at_{:x}:_{:08x}", offset, val),
        };

        if let Some(port) = active_port {
            self.do_dma(port);
        }
    }

    fn do_dma(&mut self, port: Port) {
        match self.dma.channel(port).sync() {
            Sync::LinkedList => self.do_dma_linked_list(port),
            _                => self.do_dma_block(port),
        }
    }

    fn do_dma_block(&mut self, port: Port) {
        let channel = self.dma.channel_mut(port);

        let increment = match channel.step() {
            Step::Increment => 4,
            Step::Decrement => -4i32 as u32,
        };

        let mut addr = channel.base();

        let mut remsz = match channel.transfer_size() {
            Some(n) => n,
            None => panic!("couldn't_figure_out_DMA_block_transfer_size"),
        };

        while remsz > 0 {
            let cur_addr = addr & 0x1ffffc;

            match channel.dir() {
                Direction::FromRam => {
                    let src_word = self.ram.load_32(cur_addr);

                    match port {
                        Port::Gpu => self.gpu.gp0(src_word),
                        _ => todo!("unhandled_DMA_destination_port_{}", port as u8)
                    }
                },
                Direction::ToRam => {
                    let src_word = match port {
                        Port::Otc => match remsz {
                            1 => 0xffffff,
                            _ => addr.wrapping_sub(4) & 0x1fffff,
                        },
                        _ => todo!("unhandled_DMA_source_port_{}", port as u8)
                    };

                    self.ram.store_32(cur_addr, src_word);
                }
            }

            addr = addr.wrapping_add(increment);
            remsz -= 1;
        }
        channel.done();
    }

    fn do_dma_linked_list(&mut self, port: Port) {
        let channel = self.dma.channel_mut(port);
        let mut addr = channel.base() & 0x1ffffc;

        if channel.dir() == Direction::ToRam {
            panic!("Invalid_DMA_direction_for_LinkedList_mode");
        }

        if port != Port::Gpu {
            todo!("attemped_LinkedList_DMA_on_port_{}", port as u8);
        }

        loop {
            let header = self.ram.load_32(addr);
            let mut remsz = header >> 24;

            while remsz > 0 {
                addr = (addr + 4) & 0x1ffffc;

                let command = self.ram.load_32(addr);

                self.gpu.gp0(command);

                remsz -= 1;
            }

            if header & 0x800000 != 0 {
                break;
            }

            addr = header & 0x1ffffc;
        }

        channel.done();
    }

    pub fn gpu_mut(&mut self) -> &mut Gpu {
        &mut self.gpu
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

    pub const GPU: Range = Range(0x1f801810, 0x8);
}
