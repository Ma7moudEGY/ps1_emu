use crate::{
    cpu::{decode::decode, exception::Exception::LoadAddressError},
    interconnect::{Addressable, Interconnect},
};
use exception::Exception;
use instruction::Instruction;
use icache::ICacheLine;

mod decode;
mod exception;
mod icache;
mod instruction;
mod ops;

type Op = fn(&mut Cpu, Instruction);

const CACHE_HIT: u32 = 1;
const NORMAL_FETCH: u32 = 4;

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
    icache: [ICacheLine; 256],
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
            icache: [ICacheLine::new(); 256],
        }
    }

    pub(super) fn reg(&self, index: RegisterIndex) -> u32 {
        self.regs[index.0 as usize]
    }

    pub(super) fn set_reg(&mut self, index: RegisterIndex, val: u32) {
        self.out_regs[index.0 as usize] = val;
        self.out_regs[0] = 0;
    }

    pub fn run_next_instruction(&mut self) -> u32 {
        self.current_pc = self.pc;

        if self.current_pc % 4 != 0 {
            self.exception(LoadAddressError);
            self.inter.sync(1);
            return 1;
        }

        let (instruction, cycles) = self.fetch_instruction(self.pc);

        self.pc = self.next_pc;
        self.next_pc = self.next_pc.wrapping_add(4);

        let (reg, val) = self.load;
        self.set_reg(reg, val);

        self.load = (RegisterIndex(0), 0);

        self.delay_slot = self.branch;
        self.branch = false;

        self.execute(instruction);

        self.regs = self.out_regs;
        self.inter.sync(cycles + 1);

        cycles + 1
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
            let abs_addr = addr & 0x1fffffff;
            if abs_addr < 0x1000 {
                let index = ((abs_addr >> 4) & 0xff) as usize;
                self.icache[index].write_tag(abs_addr, val.as_u32());
            }
            return;
        }
        self.inter.store(addr, val)
    }

    fn fetch_instruction(&mut self, addr: u32) -> (Instruction, u32) {
        if addr & 0xe0000000 != 0x80000000 {
            return (decode(Instruction::new(self.inter.load(addr))), NORMAL_FETCH);
        }

        let line_index = ((addr >> 4) & 0xff) as usize;
        let word_index = (addr >> 2) & 3;
        let tag = addr & 0xfffff000;
        let line = &mut self.icache[line_index];

        if line.tag() == tag && line.valid(word_index) {
            return (line.instruction(word_index), CACHE_HIT);
        }

        line.set_tag_valid(tag);

        let mut cycles = 1;
        let base = addr & !0xf;
        for i in word_index..4 {
            let raw = self.inter.load(base + (i << 2));
            line.set_instruction(i, decode(Instruction::new(raw)));
            line.set_valid(i);
            cycles += 1;
        }

        (line.instruction(word_index), cycles)
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
