use crate::cpu::{Cpu, RegisterIndex, instruction::Instruction};

pub(super) fn op_todo(_cpu: &mut Cpu, instruction: Instruction) {
    todo!("unimplemented instruction: {:08x}", instruction.raw);
}

pub(super) fn op_sll(cpu: &mut Cpu, instruction: Instruction) {
    let i = instruction.shift();
    let t = instruction.t();
    let d = instruction.d();

    let v = cpu.reg(t) << i;

    cpu.set_reg(d, v);
}

pub(super) fn op_sra(cpu: &mut Cpu, instruction: Instruction) {
    let t = instruction.t();
    let d = instruction.d();
    let i = instruction.shift();

    let v = (cpu.reg(t) as i32) >> i;

    cpu.set_reg(d, v as u32);
}

pub(super) fn op_div(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();

    let n = cpu.reg(s) as i32;
    let d = cpu.reg(t) as i32;

    if d == 0 {
        cpu.hi = n as u32;

        if n >= 0 {
            cpu.lo = 0xffffffff;
        } else {
            cpu.lo = 1;
        }
    } else if n as u32 == 0x80000000 && d == -1 {
        cpu.hi = 0;
        cpu.lo = 0x80000000;
    } else {
        cpu.hi = (n % d) as u32;
        cpu.lo = (n / d) as u32;
    }
}

pub(super) fn op_add(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let d = instruction.d();

    let s = cpu.reg(s) as i32;
    let t = cpu.reg(t) as i32;

    let v = match s.checked_add(t) {
        Some(v) => v as u32,
        None => todo!("ADD_overflow"),
    };

    cpu.set_reg(d, v);
}

pub(super) fn op_addu(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let d = instruction.d();

    let v = cpu.reg(s).wrapping_add(cpu.reg(t));

    cpu.set_reg(d, v);
}

pub(super) fn op_subu(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let d = instruction.d();

    let v = cpu.reg(s).wrapping_sub(cpu.reg(t));

    cpu.set_reg(d, v);
}

pub(super) fn op_and(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let d = instruction.d();

    let v = cpu.reg(s) & cpu.reg(t);

    cpu.set_reg(d, v);
}

pub(super) fn op_sltu(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let d = instruction.d();

    let v = cpu.reg(s) < cpu.reg(t);

    cpu.set_reg(d, v as u32);
}

pub(super) fn op_or(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let d = instruction.d();

    let v = cpu.reg(s) | cpu.reg(t);

    cpu.set_reg(d, v);
}

pub(super) fn op_addiu(cpu: &mut Cpu, instruction: Instruction) {
    let i = instruction.imm_se();
    let t = instruction.t();
    let s = instruction.s();

    let v = cpu.reg(s).wrapping_add(i);

    cpu.set_reg(t, v);
}

pub(super) fn op_addi(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm_se() as i32;

    let s = cpu.reg(s) as i32;

    let v = match s.checked_add(i) {
        Some(v) => v as u32,
        None => todo!("ADDI_overflow"),
    };

    cpu.set_reg(t, v);
}

pub(super) fn op_lui(cpu: &mut Cpu, instruction: Instruction) {
    let t = instruction.t();
    let i = instruction.imm();

    let v = i << 16;

    cpu.set_reg(t, v);
}

pub(super) fn op_andi(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm();

    let v = cpu.reg(s) & i;

    cpu.set_reg(t, v);
}

pub(super) fn op_ori(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm();

    let v = cpu.reg(s) | i;

    cpu.set_reg(t, v);
}

pub(super) fn op_lw(cpu: &mut Cpu, instruction: Instruction) {
    if cpu.sr & 0x10000 != 0 {
        println!("ignoring_load_while_cache_is_isolated");
        return;
    }

    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm_se();

    let addr = cpu.reg(s).wrapping_add(i);

    let v = cpu.read_word(addr);

    cpu.load = (t, v);
}

pub(super) fn op_sw(cpu: &mut Cpu, instruction: Instruction) {
    if cpu.sr & 0x10000 != 0 {
        println!("ignoring_store_while_cache_is_isolated");
        return;
    }
    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm_se();

    let addr = cpu.reg(s).wrapping_add(i);
    let v = cpu.reg(t);

    cpu.store_32(addr, v);
}

pub(super) fn op_j(cpu: &mut Cpu, instruction: Instruction) {
    let i = instruction.imm_jump();

    cpu.pc = (cpu.pc & 0xf0000000) | (i << 2);
}

pub(super) fn op_jal(cpu: &mut Cpu, instruction: Instruction) {
    let ra = cpu.pc;

    cpu.set_reg(RegisterIndex(31), ra);

    op_j(cpu, instruction);
}

pub(super) fn op_sh(cpu: &mut Cpu, instruction: Instruction) {
    if cpu.sr & 0x10000 != 0 {
        println!("ignoring_store_while_cache_is_isolated");
        return;
    }

    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm_se();

    let addr = cpu.reg(s).wrapping_add(i);
    let v = cpu.reg(t);

    cpu.store_16(addr, v as u16);
}

pub(super) fn op_bne(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm_se();

    if cpu.reg(s) != cpu.reg(t) {
        cpu.branch(i);
    }
}

pub(super) fn op_sb(cpu: &mut Cpu, instruction: Instruction) {
    if cpu.sr & 0x10000 != 0 {
        println!("ignoring_store_while_cache_is_isolated");
        return;
    }

    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm_se();

    let addr = cpu.reg(s).wrapping_add(i);
    let v = cpu.reg(t);

    cpu.store_8(addr, v as u8);
}

pub(super) fn op_jr(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();

    cpu.pc = cpu.reg(s);
}

pub(super) fn op_lb(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm_se();

    let addr = cpu.reg(s).wrapping_add(i);

    let v = cpu.load_8(addr) as i8;

    cpu.load = (t, v as u32);
}

pub(super) fn op_beq(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm_se();

    if cpu.reg(s) == cpu.reg(t) {
        cpu.branch(i);
    }
}

pub(super) fn op_bxx(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let i = instruction.imm_se();

    let instruction = instruction.raw;

    let is_begz = (instruction >> 16) & 1;
    let is_link = (instruction >> 20) & 1 != 0;

    let v = cpu.reg(s) as i32;

    let test = (v < 0) as u32;

    let test = test ^ is_begz;

    if test != 0 {
        if is_link {
            let ra = cpu.pc;

            cpu.set_reg(RegisterIndex(31), ra);
        }

        cpu.branch(i);
    }
}

pub(super) fn op_bgtz(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let i = instruction.imm_se();

    let v = cpu.reg(s) as i32;

    if v > 0 {
        cpu.branch(i);
    }
}

pub(super) fn op_blez(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let i = instruction.imm_se();

    let v = cpu.reg(s) as i32;

    if v <= 0 {
        cpu.branch(i);
    }
}

pub(super) fn op_lbu(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm_se();

    let addr = cpu.reg(s).wrapping_add(i);

    let v = cpu.load_8(addr);

    cpu.load = (t, v as u32);
}

pub(super) fn op_jalr(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let d = instruction.d();

    let ra = cpu.pc;

    cpu.set_reg(d, ra);

    cpu.pc = cpu.reg(s);
}

pub(super) fn op_slti(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm_se() as i32;

    let v = (cpu.reg(s) as i32) < i;

    cpu.set_reg(t, v as u32);
}

pub(super) fn op_mflo(cpu: &mut Cpu, instruction: Instruction) {
    let d = instruction.d();

    let lo = cpu.lo;

    cpu.set_reg(d, lo);
}

pub(super) fn op_srl(cpu: &mut Cpu, instruction: Instruction) {
    let t = instruction.t();
    let d = instruction.d();
    let i = instruction.shift();

    let v = cpu.reg(t) >> i;

    cpu.set_reg(d, v);
}

pub(super) fn op_sltiu(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm_se();

    let v = cpu.reg(s) < i;

    cpu.set_reg(t, v as u32);
}

pub(super) fn op_divu(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();

    let n = cpu.reg(s);
    let d = cpu.reg(t);

    if d == 0 {
        cpu.hi = n;
        cpu.lo = 0xffffffff;
    } else {
        cpu.hi = n % d;
        cpu.lo = n /d;
    }
}

pub(super) fn op_mfhi(cpu: &mut Cpu, instruction: Instruction) {
    let d = instruction.d();

    let hi = cpu.hi;

    cpu.set_reg(d, hi);
}

pub(super) fn op_slt(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let d = instruction.d();

    let s = cpu.reg(s) as i32;
    let t = cpu.reg(t) as i32;
    
    let v = s < t;

    cpu.set_reg(d, v as u32);
}

// COP INSTRUCTIONS

pub(super) fn op_cop0(cpu: &mut Cpu, instruction: Instruction) {
    match instruction.cop_opcode() {
        0b00100 => op_mtc0(cpu, instruction),
        0b00000 => op_mfc0(cpu, instruction),
        _ => todo!("unhandled_cop0_instruction_{:08x}", instruction.raw),
    }
}

pub(super) fn op_mtc0(cpu: &mut Cpu, instruction: Instruction) {
    let cpu_r = instruction.t();
    let cop_r = instruction.d().0;

    let v = cpu.reg(cpu_r);

    match cop_r {
        3 | 5 | 6 | 7 | 9 | 11 => {
            if v != 0 {
                todo!("unhandled_write_to_cop0r{}", cop_r)
            }
        }
        12 => cpu.sr = v,
        13 => {
            if v != 0 {
                todo!("unhandled_write_to_CAUSE_register")
            }
        }
        _ => todo!("unhandled_cop0_register_{:08x}", cop_r),
    }
}

pub(super) fn op_mfc0(cpu: &mut Cpu, instruction: Instruction) {
    let cpu_r = instruction.t();
    let cop_r = instruction.d().0;

    let v = match cop_r {
        12 => cpu.sr,
        13 => todo!("unhandled_read_from_CAUSE_register"),
        _ => todo!("uhandled_read_from_cop0r{:08x}", cop_r),
    };

    cpu.load = (cpu_r, v)
}
