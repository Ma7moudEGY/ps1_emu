use crate::{cpu::{decode::decode, exception::Exception::LoadAddressError}, interconnect::Interconnect};
use instruction::Instruction;
use exception::Exception;

mod decode;
mod exception;
mod icache;
mod instruction;
mod ops;

type Op = fn(&mut Cpu, Instruction);

#[derive(Clone, Copy)]
pub struct RegisterIndex(u32);

pub struct Cpu {
    pc: u32,
    next_pc: u32,
    regs: [u32; 32],
    out_regs: [u32; 32],
    load: (RegisterIndex, u32),
    inter: Interconnect,
    next_instruction: Instruction,
    hi: u32,
    lo: u32,
    sr: u32,
    current_pc: u32,
    cause: u32,
    epc: u32,
    branch: bool,
    delay_slot: bool,
}

impl Cpu {
    pub fn new(inter: Interconnect) -> Cpu {
        let mut regs = [0xdeadbeef; 32];
        regs[0] = 0;
        let pc  = 0xbfc00000;

        Cpu {
            pc: pc,
            next_pc: pc.wrapping_add(4),
            regs: regs,
            out_regs: regs,
            load: (RegisterIndex(0), 0),
            inter: inter,
            next_instruction: decode(Instruction::new(0x0)),
            hi: 0xdeadbeef,
            lo: 0xdeadbeef, 
            sr: 0,
            current_pc: pc,
            cause: 0,
            epc: 0,
            branch: false,
            delay_slot: false,
        }
    }

    pub(super) fn reg(&self, index: RegisterIndex) -> u32 {
        self.regs[index.0 as usize]
    }

    pub(super) fn set_reg(&mut self, index: RegisterIndex, val: u32) {
        self.out_regs[index.0 as usize] = val;
        self.out_regs[0] = 0;
    }

    pub fn run_next_instruction(&mut self) {
        self.current_pc = self.pc;

        if self.current_pc % 4 != 0 {
            self.exception(LoadAddressError);
            return;
        }

        let instruction = self.fetch_instruction(self.pc);

        self.pc = self.next_pc;
        self.next_pc = self.next_pc.wrapping_add(4);

        let (reg, val) = self.load;
        self.set_reg(reg, val);

        self.load = (RegisterIndex(0), 0);

        self.delay_slot = self.branch;
        self.branch = false;

        self.execute(instruction);

        self.regs = self.out_regs;
    }

    fn branch(&mut self, offset: u32) {
        let offset = offset << 2;

        let mut pc = self.next_pc;

        pc = pc.wrapping_add(offset);

        pc = pc .wrapping_sub(4);

        self.branch = true;
        self.next_pc = pc;
    }

    fn load_8(&self, addr: u32) -> u8 {
        self.inter.load_8(addr)
    }

    fn load_16(&self, addr: u32) -> u16 {
        self.inter.load_16(addr)
    }

    fn fetch_instruction(&self, addr: u32) -> Instruction {
        decode(Instruction::new(self.inter.load_32(addr)))
    }

    fn read_word(&self, addr: u32) -> u32 {
        self.inter.load_32(addr)
    }

    fn store_8(&mut self, addr: u32, val: u8) {
        self.inter.store_8(addr, val);
    }

    fn store_16(&mut self, addr: u32, val: u16) {
        self.inter.store_16(addr, val);
    }

    fn store_32(&mut self, addr: u32, val: u32) {
        self.inter.store_32(addr, val);
    }

    fn execute(&mut self, instruction: Instruction) {
        (instruction.execute)(self, instruction);
    }

    fn exception(&mut self, cause: Exception) {
        let handler: u32 = match self.sr & (1 << 22) != 0 {
            true    => 0xbfc00180,
            false   => 0x80000080,
        };

        let mode = self.sr & 0x3f;
        self.sr &= !0x3f;
        self.sr |= (mode << 2) & 0x3f;

        self.cause = (cause as u32) << 2;

        self.epc = self.current_pc;

        if self.delay_slot {
            self.epc = self.epc.wrapping_sub(4);
            self.cause |= 1 << 31;
        }

        self.pc = handler;
        self.next_pc = self.pc.wrapping_add(4);

    }
}
