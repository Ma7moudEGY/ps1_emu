#[derive(Clone, Copy)]
pub struct Instruction {
    pub raw: u32,
    pub execute: Op,
}

use crate::cpu::{Op, ops::op_illegal};

use super::RegisterIndex;

impl Instruction {
    pub fn new(raw: u32) -> Instruction {
        Instruction {
            raw,
            execute: op_illegal,
        }
    }

    pub fn function(&self) -> u32 {
        let op = self.raw;

        op >> 26
    }

    pub fn t(&self) -> RegisterIndex {
        let op = self.raw;

        RegisterIndex((op >> 16) & 0x1f)
    }

    pub fn s(&self) -> RegisterIndex {
        let op = self.raw;

        RegisterIndex((op >> 21) & 0x1f)
    }

    pub fn d(&self) -> RegisterIndex {
        let op = self.raw;

        RegisterIndex((op >> 11) & 0x1f)
    }

    pub fn sub_function(&self) -> u32 {
        let op = self.raw;

        op & 0x3f
    }

    pub fn shift(&self) -> u32 {
        let op = self.raw;

        (op >> 6) & 0x1f
    }

    pub fn imm(&self) -> u32 {
        let op = self.raw;

        op & 0xffff
    }

    pub fn imm_se(&self) -> u32 {
        let op = self.raw;

        let v = (op & 0xffff) as i16;

        v as u32
    }

    pub fn imm_jump(&self) ->u32 {
        let op = self.raw;

        op & 0x3ffffff
    }

    pub fn cop_opcode(&self) -> u32 {
        let op = self.raw;

        (op >> 21) & 0x1f
    }
}