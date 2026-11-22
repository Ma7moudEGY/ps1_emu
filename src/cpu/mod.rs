use crate::{
    cpu::{decode::decode, exception::Exception::LoadAddressError},
    interconnect::{Addressable, Interconnect},
};
use exception::Exception;
use instruction::Instruction;

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
        let pc = 0xbfc00000;

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

        pc = pc.wrapping_sub(4);

        self.branch = true;
        self.next_pc = pc;
    }

    fn load<T: Addressable>(&self, addr: u32) -> T {
        self.inter.load(addr)
    }

    fn store<T: Addressable>(&mut self, addr: u32, val: T) {
        if self.sr & 0x10000 != 0 {
            println!("ignoring_store_while_cache_is_isolated");
            return;
        }
        self.inter.store(addr, val)
    }

    fn fetch_instruction(&self, addr: u32) -> Instruction {
        decode(Instruction::new(self.inter.load(addr)))
    }

    fn read_word(&self, addr: u32) -> u32 {
        self.inter.load(addr)
    }

    fn execute(&mut self, instruction: Instruction) {
        (instruction.execute)(self, instruction);
    }

    fn exception(&mut self, cause: Exception) {
        let handler: u32 = match self.sr & (1 << 22) != 0 {
            true => 0xbfc00180,
            false => 0x80000080,
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

    pub fn interconnect_mut(&mut self) -> &mut Interconnect {
        &mut self.inter
    }
}
