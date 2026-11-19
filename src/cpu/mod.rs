use crate::{cpu::{decode::decode}, interconnect::Interconnect};
use instruction::Instruction;

mod decode;
mod exceptions;
mod icache;
mod instruction;
mod ops;

type Op = fn(&mut Cpu, Instruction);

#[derive(Clone, Copy)]
pub struct RegisterIndex(u32);

pub struct Cpu {
    pc: u32,
    regs: [u32; 32],
    out_regs: [u32; 32],
    load: (RegisterIndex, u32),
    inter: Interconnect,
    next_instruction: Instruction,
    sr: u32,
    hi: u32,
    lo: u32,
}

impl Cpu {
    pub fn new(inter: Interconnect) -> Cpu {
        let mut regs = [0xdeadbeef; 32];
        regs[0] = 0;

        Cpu {
            pc: 0xbfc00000,
            regs: regs,
            out_regs: regs,
            load: (RegisterIndex(0), 0),
            inter: inter,
            next_instruction: decode(Instruction::new(0x0)),
            sr: 0,
            hi: 0xdeadbeef,
            lo: 0xdeadbeef, 
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
        let pc = self.pc;

        let instruction = self.next_instruction;
        self.next_instruction = self.fetch_instruction(pc);

        self.pc = pc.wrapping_add(4);

        let (reg, val) = self.load;
        self.set_reg(reg, val);

        self.load = (RegisterIndex(0), 0);

        self.execute(instruction);

        self.regs = self.out_regs;
    }

    fn branch(&mut self, offset: u32) {
        let offset = offset << 2;

        let mut pc = self.pc;

        pc = pc.wrapping_add(offset);

        pc = pc .wrapping_sub(4);

        self.pc = pc;
    }

    fn load_8(&self, addr: u32) -> u8 {
        self.inter.load_8(addr)
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
}
