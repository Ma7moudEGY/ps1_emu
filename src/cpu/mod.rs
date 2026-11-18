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
    next_instruction: Instruction,
}

impl Cpu {
    pub fn new(inter: Interconnect) -> Cpu {
        let mut regs = [0xdeadbeef; 32];
        regs[0] = 0;

        Cpu {
            pc: 0xbfc00000,
            regs: regs,
            inter: inter,
            next_instruction: Instruction(0x0),
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

        let instruction = self.next_instruction;
        self.next_instruction = self.load_32(pc);

        self.pc = pc.wrapping_add(4);

        self.decode_and_execute(instruction);
    }

    fn load_32(&self, addr: u32) -> Instruction {
        Instruction(self.inter.load_32(addr))
    }

    fn store_32(&mut self, addr: u32, val: u32) {
        self.inter.store_32(addr, val);
    }

    fn decode_and_execute(&mut self, instruction: Instruction) {
        match instruction.function() {
            0b000000    =>  {
                let sub_funct = instruction.sub_function();
                match sub_funct {
                    0x00    => self.op_sll(instruction),
                    0x25    => self.op_or(instruction),
                    _       => panic!("unknown_sub_function_{:06x}", sub_funct)
                }
            },
            0b000010    => self.op_j(instruction),
            0b001001    => self.op_addiu(instruction),
            0b001111    => self.op_lui(instruction),
            0b001101    => self.op_ori(instruction),
            0b101011    => self.op_sw(instruction),
            _           => panic!("unhandled_instruction_at_{:08x}", instruction.0),
        }
    }

    fn op_sll(&mut self, instruction: Instruction) {
        let i = instruction.shift();
        let t = instruction.t();
        let d = instruction.d();

        let v = self.reg(t) << i;

        self.set_reg(d, v);
    }

    fn op_or(&mut self, instruction: Instruction) {
        let d = instruction.d();
        let s = instruction.s();
        let t = instruction.t();

        let v = self.reg(s) | self.reg(t);

        self.set_reg(d, v);
    }

    fn op_addiu(&mut self, instruction: Instruction) {
        let i = instruction.imm();
        let t = instruction.t();
        let s = instruction.s();

        let v = self.reg(s).wrapping_add(i);

        self.set_reg(t, v);
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

    fn op_sw(&mut self, instruction: Instruction) {
        let i = instruction.imm_se();
        let t = instruction.t();
        let s = instruction.s();

        let addr = self.reg(s).wrapping_add(i);
        let v = self.reg(t);

        self.store_32(addr, v);
    }

    fn op_j(&mut self, instruction: Instruction) {
        let i = instruction.imm_jump();

        self.pc = (self.pc &0xf0000000) | (i << 2);
    }
}
