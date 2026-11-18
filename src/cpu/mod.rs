use core::panic;

use crate::interconnect::Interconnect;
use instructions::Instruction;

mod decode;
mod exceptions;
mod icache;
mod instructions;

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
            next_instruction: Instruction(0x0),
            sr: 0,
        }
    }

    fn reg(&self, index: RegisterIndex) -> u32 {
        self.regs[index.0 as usize]
    }

    fn set_reg(&mut self, index: RegisterIndex, val: u32) {
        self.out_regs[index.0 as usize] = val;
        self.out_regs[0] = 0;
    }

    pub fn run_next_instruction(&mut self) {
        let pc = self.pc;

        let instruction = self.next_instruction;
        self.next_instruction = self.load_32(pc);

        self.pc = pc.wrapping_add(4);

        let (reg, val) = self.load;
        self.set_reg(reg, val);

        self.load = (RegisterIndex(0), 0);

        self.decode_and_execute(instruction);

        self.regs = self.out_regs;
    }

    fn branch(&mut self, offset: u32) {
        let offset = offset << 2;

        let mut pc = self.pc;

        pc = pc.wrapping_add(offset);

        pc = pc .wrapping_sub(4);

        self.pc = pc;
    }

    fn load_32(&self, addr: u32) -> Instruction {
        Instruction(self.inter.load_32(addr))
    }

    fn read_word(&self, addr: u32) -> u32 {
        self.inter.load_32(addr)
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
                    0x21    => self.op_addu(instruction),
                    0x25    => self.op_or(instruction),
                    0x2b    => self.op_sltu(instruction),
                    _       => panic!("unknown_sub_function_{:06x}", sub_funct)
                }
            },
            0b000010    => self.op_j(instruction),
            0b000101    => self.op_bne(instruction),
            0b001000    => self.op_addi(instruction),
            0b001001    => self.op_addiu(instruction),
            0b001111    => self.op_lui(instruction),
            0b001101    => self.op_ori(instruction),
            0b010000    => self.op_cop0(instruction),
            0b100011    => self.op_lw(instruction),
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

    fn op_addu(&mut self, instruction: Instruction) {
        let s = instruction.s();
        let t = instruction.t();
        let d = instruction.d();

        let v = self.reg(s).wrapping_add(self.reg(t));

        self.set_reg(d, v);
    }

    fn op_sltu(&mut self, instruction: Instruction) {
        let s = instruction.s();
        let t = instruction.t();
        let d = instruction.d();

        let v = self.reg(s) < self.reg(t);

        self.set_reg(d, v as u32);
    }

    fn op_or(&mut self, instruction: Instruction) {
        let s = instruction.s();
        let t = instruction.t();
        let d = instruction.d();

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

    fn op_addi(&mut self, instruction: Instruction) {
        let s = instruction.s();
        let t = instruction.t();
        let i = instruction.imm_se() as i32;

        let s = self.reg(s) as i32;

        let v = match s.checked_add(i) {
            Some(v) => v as u32,
            None => panic!("ADDI_overflow"),
        };

        self.set_reg(t, v);
    }

    fn op_lui(&mut self, instruction: Instruction) {
        let t = instruction.t();
        let i = instruction.imm();

        let v = i << 16;

        self.set_reg(t, v);
    }

    fn op_ori(&mut self, instruction: Instruction) {
        let s = instruction.s();
        let t = instruction.t();
        let i = instruction.imm();

        let v = self.reg(s) | i;

        self.set_reg(t, v);
    }

    fn op_lw(&mut self, instruction: Instruction) {
        if self.sr & 0x10000 != 0 {
            println!("ignoring_load_while_cache_is_isolated");
            return;
        }

        let s = instruction.s();
        let t = instruction.t();
        let i = instruction.imm_se();

        let addr = self.reg(s).wrapping_add(i);

        let v = self.read_word(addr);

        self.load = (t, v);
    }

    fn op_sw(&mut self, instruction: Instruction) {
        if self.sr & 0x10000 != 0 {
            println!("ignoring_store_while_cache_is_isolated");
            return;
        }
        let s = instruction.s();
        let t = instruction.t();
        let i = instruction.imm_se();

        let addr = self.reg(s).wrapping_add(i);
        let v = self.reg(t);

        self.store_32(addr, v);
    }

    fn op_j(&mut self, instruction: Instruction) {
        let i = instruction.imm_jump();

        self.pc = (self.pc &0xf0000000) | (i << 2);
    }

    fn op_bne(&mut self, instruction: Instruction) {
        let t = instruction.t();
        let s = instruction.s();
        let i = instruction.imm_se();

        if self.reg(s) != self.reg(t) {
            self.branch(i);
        }
    }

    // COP INSTRUCTIONS

    fn op_cop0(&mut self, instruction: Instruction) {
        match instruction.cop_opcode() {
            0b00100 => self.op_mtc0(instruction),
            _       => panic!("unhandled_cop0_instruction_{:08x}", instruction.0)
        }
    }

    fn op_mtc0(&mut self, instruction: Instruction) {
        let cpu_r = instruction.t();
        let cop_r = instruction.d().0;

        let v = self.reg(cpu_r);

        match cop_r {
            3 | 5 | 6 | 7 | 9 | 11 =>
                if v != 0 {
                    panic!("unhandled_write_to_cop0r{}", cop_r)
                },
            12 => self.sr = v,
            13 => 
                if v != 0 {
                    panic!("unhandled_write_to_CAUSE_register")
                },
            _ => panic!("unhandled_cop0_register_{:08x}", cop_r),
        }
    }
}
