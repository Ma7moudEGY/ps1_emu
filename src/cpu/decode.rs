use super::instruction::Instruction;
use super::ops::*;

pub(super) fn decode(mut instruction: Instruction) -> Instruction {
    instruction.execute = match instruction.function() {
            0b000000    =>  {
                let sub_funct = instruction.sub_function();
                match sub_funct {
                    0x00    => op_sll,
                    0x02    => op_srl,
                    0x03    => op_sra,
                    0x08    => op_jr,
                    0x09    => op_jalr,
                    0x10    => op_mfhi,
                    0x12    => op_mflo,
                    0x1a    => op_div,
                    0x1b    => op_divu,
                    0x20    => op_add,
                    0x21    => op_addu,
                    0x23    => op_subu,
                    0x24    => op_and,
                    0x25    => op_or,
                    0x2a    => op_slt,
                    0x2b    => op_sltu,
                    _       => todo!("unknown_sub_function_{:06x}", sub_funct)
                }
            },
            0b000001    => op_bxx,
            0b000010    => op_j,
            0b000011    => op_jal,
            0b000100    => op_beq,
            0b000101    => op_bne,
            0b000110    => op_blez,
            0b000111    => op_bgtz,
            0b001000    => op_addi,
            0b001001    => op_addiu,
            0b001010    => op_slti,
            0b001011    => op_sltiu,
            0b001111    => op_lui,
            0b001100    => op_andi,
            0b001101    => op_ori,
            0b010000    => op_cop0,
            0b100000    => op_lb,
            0b100011    => op_lw,
            0b100100    => op_lbu,
            0b101000    => op_sb,
            0b101001    => op_sh,
            0b101011    => op_sw,
            _           => op_todo,
        };

        instruction
}