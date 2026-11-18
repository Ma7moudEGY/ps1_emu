use crate::interconnect::Interconnect;
use instructions::Instruction;

mod decode;
mod exceptions;
mod icache;
mod instructions;

pub struct Cpu {
    pc: u32,
    regs: [u32; 32],
    inter: Interconnect,
}

impl Cpu {
    pub fn new(inter: Interconnect) -> Cpu {
        let mut regs = [0xdeadbeef; 32];
        regs[0] = 0;

        Cpu {
            pc: 0xbfc00000,
            regs: regs,
            inter: inter,
        }
    }

    fn reg(&self, index: u32) -> u32 {
        self.regs[index as usize]
    }

    fn set_reg(&mut self, index: u32, val: u32) {
        self.regs[index as usize] = val;
        self.regs[0] = 0;
    }

    pub fn run_next_instruction(&mut self) {
        let pc = self.pc;

        let instruction = self.load_32(pc);

        self.pc = pc.wrapping_add(4);

        self.decode_and_execute(instruction);
    }

    fn load_32(&self, addr: u32) -> Instruction {
        Instruction(self.inter.load_32(addr))
    }

    fn decode_and_execute(&mut self, instruction: Instruction) {
        match instruction.function() {
            0b001111    => self.op_lui(instruction),
            0b001101    => self.op_ori(instruction),
            _           => panic!("unhandled_instruction_at_{:x}", instruction.0),
        }
    }

    fn op_lui(&mut self, instruction: Instruction) {
        let i = instruction.imm();
        let t = instruction.t();

        let v = i << 16;

        self.set_reg(t, v);
    }

    fn op_ori(&mut self, instruction: Instruction) {
        let i = instruction.imm();
        let t = instruction.t();
        let s = instruction.s();

        let v = self.reg(s) | i;

        self.set_reg(t, v);
    }
}
