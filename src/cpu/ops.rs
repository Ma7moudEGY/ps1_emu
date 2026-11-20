use crate::cpu::{Cpu, RegisterIndex, exception::Exception::{self, CoprocessorError, StoreAddressError}, instruction::{Instruction}};

pub(super) fn op_illegal(cpu: &mut Cpu, instruction: Instruction) {
    println!("IllegalInstruction_{}!", instruction.raw);
    cpu.exception(Exception::IllegalInstruction);
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

    match s.checked_add(t) {
        Some(v) => cpu.set_reg(d, v as u32),
        None => cpu.exception(Exception::Overflow),
    };
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

    match s.checked_add(i) {
        Some(v) => cpu.set_reg(t, v as u32),
        None => cpu.exception(Exception::Overflow),
    };
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
    if addr % 4 == 0 {
        let v = cpu.read_word(addr);
        cpu.load = (t, v);
    } else {
        cpu.exception(Exception::LoadAddressError);
    }
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

    if addr % 4 == 0 {
        let v = cpu.reg(t);
        cpu.store_32(addr, v);
    } else {
        cpu.exception(Exception::StoreAddressError);
    }
}

pub(super) fn op_j(cpu: &mut Cpu, instruction: Instruction) {
    let i = instruction.imm_jump();
    cpu.branch = true;

    cpu.next_pc = (cpu.pc & 0xf0000000) | (i << 2);
}

pub(super) fn op_jal(cpu: &mut Cpu, instruction: Instruction) {
    let ra = cpu.next_pc;

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

    if addr % 2 == 0 {
        let v = cpu.reg(t);
        cpu.store_16(addr, v as u16);
    } else {
        cpu.exception(StoreAddressError);
    }

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

    cpu.branch = true;
    cpu.next_pc = cpu.reg(s);
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
            let ra = cpu.next_pc;

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

    let ra = cpu.next_pc;

    cpu.set_reg(d, ra);

    cpu.branch = true;
    cpu.next_pc = cpu.reg(s);
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

pub(super) fn op_mtlo(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();

    cpu.lo = cpu.reg(s);
}

pub(super) fn op_mthi(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();

    cpu.hi = cpu.reg(s);
}

pub(super) fn op_lhu(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm_se();

    let addr = cpu.reg(s).wrapping_add(i);

    if addr % 2 == 0 {
        let v = cpu.load_16(addr);
        cpu.load = (t, v as u32);
    } else {
        cpu.exception(Exception::LoadAddressError);
    }
}

pub(super) fn op_sllv(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let d = instruction.d();

    let v = cpu.reg(t) << (cpu.reg(s) & 0x1f);

    cpu.load = (d, v);
}

pub(super) fn op_lh(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm_se();

    let addr = cpu.reg(s).wrapping_add(i);

    let v = cpu.load_16(addr) as i16;

    cpu.load = (t, v as u32);
}

pub(super) fn op_nor(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let d = instruction.d();

    let v = !(cpu.reg(s) | cpu.reg(t));

    cpu.set_reg(d, v);
}

pub(super) fn op_srav(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let d = instruction.d();

    let v = (cpu.reg(t) as i32) >> (cpu.reg(s) & 0x1f);

    cpu.set_reg(d, v as u32);
}

pub(super) fn op_srlv(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let d = instruction.d();

    let v = cpu.reg(t) >> (cpu.reg(s) & 0x1f);

    cpu.load = (d, v);
}

pub(super) fn op_multu(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();

    let a = cpu.reg(s) as u64;
    let b = cpu.reg(t) as u64;

    let v = a * b;

    cpu.hi = (v >> 32) as u32;
    cpu.lo = v as u32;
}

pub(super) fn op_xor(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let d = instruction.d();

    let v = cpu.reg(s) ^ cpu.reg(t);

    cpu.set_reg(d, v);
}

pub(super) fn op_break(cpu: &mut Cpu, _instruction: Instruction) {
    cpu.exception(Exception::Break);
}

pub(super) fn op_mult(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();

    let a = (cpu.reg(s) as i32) as i64;
    let b = (cpu.reg(t) as i32) as i64;

    let v = (a * b) as u64;

    cpu.hi = (v >> 32) as u32;
    cpu.lo = v as u32;
}

pub(super) fn op_sub(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let d = instruction.d();

    let s = cpu.reg(s) as i32;
    let t = cpu.reg(t) as i32;

    match s.checked_sub(t) {
        Some(v) => cpu.set_reg(d, v as u32),
        None => cpu.exception(Exception::Overflow),
    }
}

pub(super) fn op_xori(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm();

    let v = cpu.reg(s) ^ i;

    cpu.set_reg(t, v);
}

pub(super) fn op_lwl(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm_se();

    let addr = cpu.reg(s).wrapping_add(i);

    let cur_v = cpu.out_regs[t.0 as usize];

    let aligned_addr = addr & !3;
    let aligned_word = cpu.load_32(aligned_addr);

    let v = match addr & 3 {
        0 => (cur_v & 0x00ffffff) | (aligned_word << 24),
        1 => (cur_v & 0x0000ffff) | (aligned_word << 16),
        2 => (cur_v & 0x000000ff) | (aligned_word << 8),
        3 => aligned_word,                               // the guide does (cur_v & 0x00000000) | (aligned_word << 0) which just returns the aligned word
        _ => unreachable!(),
    };

    cpu.load = (t, v);
}

pub(super) fn op_lwr(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm_se();

    let addr = cpu.reg(s).wrapping_add(i);

    let cur_v = cpu.out_regs[t.0 as usize];

    let aligned_addr = addr & !3;
    let aligned_word = cpu.load_32(aligned_addr);

    let v = match addr & 3 {
        3 => (cur_v & 0xff000000) | (aligned_word >> 24),
        2 => (cur_v & 0xffff0000) | (aligned_word >> 16),
        1 => (cur_v & 0xffffff00) | (aligned_word >> 8),
        0 => aligned_word,                               // the guide does (cur_v & 0x00000000) | (aligned_word >> 0) which just returns the aligned word
        _ => unreachable!(),
    };

    cpu.load = (t, v);
}

pub(super) fn op_swl(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm_se();

    let addr = cpu.reg(s).wrapping_add(i);
    let v = cpu.reg(t);

    let aligned_addr = addr & !3;
    let cur_mem = cpu.load_32(aligned_addr);

    let mem = match addr & 3 {
        0 => (cur_mem & 0xffffff00) | (v >> 24),
        1 => (cur_mem & 0xffff0000) | (v >> 16),
        2 => (cur_mem & 0xff000000) | (v >> 8),
        3 => v,                                 // same as the two above (cur_mem & 0x00000000 | (v >> 0))
        _ => unreachable!(),
    };

    cpu.store_32(aligned_addr, mem);
}

pub(super) fn op_swr(cpu: &mut Cpu, instruction: Instruction) {
    let s = instruction.s();
    let t = instruction.t();
    let i = instruction.imm_se();

    let addr = cpu.reg(s).wrapping_add(i);
    let v = cpu.reg(t);

    let aligned_addr = addr & !3;
    let cur_mem = cpu.load_32(aligned_addr);

    let mem = match addr & 3 {
        3 => (cur_mem & 0x00ffffff) | (v << 24),
        2 => (cur_mem & 0x0000ffff) | (v << 16),
        1 => (cur_mem & 0x000000ff) | (v << 8),
        0 => v,                                 // same as the two above (cur_mem & 0x00000000 | (v >> 0))
        _ => unreachable!(),
    };

    cpu.store_32(aligned_addr, mem);
}

// COP INSTRUCTIONS

pub(super) fn op_cop0(cpu: &mut Cpu, instruction: Instruction) {
    match instruction.cop_opcode() {
        0b00000 => op_mfc0(cpu, instruction),
        0b00100 => op_mtc0(cpu, instruction),
        0b10000 => op_ref(cpu, instruction),
        _ => todo!("unhandled_cop0_instruction_{:08x}", instruction.raw),
    }
}

pub(super) fn op_cop1(cpu: &mut Cpu, _instruction: Instruction) {
    cpu.exception(CoprocessorError);
}

pub(super) fn op_cop2(_cpu: &mut Cpu, _instruction: Instruction) {
    todo!("unhandled_GTE_instruction_{}", _instruction.raw);
}

pub(super) fn op_cop3(cpu: &mut Cpu, _instruction: Instruction) {
    cpu.exception(CoprocessorError);
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
        13 => cpu.cause,
        14 => cpu.epc,
        _ => todo!("uhandled_read_from_cop0r{:08x}", cop_r),
    };

    cpu.load = (cpu_r, v)
}

pub(super) fn op_ref(cpu: &mut Cpu, instruction: Instruction) {
    if instruction.raw & 0x3f != 0b010000 {
        panic!("invaild_cop0_instruction:{}", instruction.raw);
    }

    let mode = cpu.sr & 0x3f;
    cpu.sr &= !0x3f;
    cpu.sr |= mode >> 2;
}

pub(super) fn op_lwc0(cpu: &mut Cpu, _instruction: Instruction) {
    cpu.exception(CoprocessorError);
}

pub(super) fn op_lwc1(cpu: &mut Cpu, _instruction: Instruction) {
    cpu.exception(CoprocessorError);
}

pub(super) fn op_lwc2(cpu: &mut Cpu, _instruction: Instruction) {
    cpu.exception(CoprocessorError);
}

pub(super) fn op_lwc3(cpu: &mut Cpu, _instruction: Instruction) {
    cpu.exception(CoprocessorError);
}

pub(super) fn op_swc0(cpu: &mut Cpu, _instruction: Instruction) {
    cpu.exception(CoprocessorError);
}

pub(super) fn op_swc1(cpu: &mut Cpu, _instruction: Instruction) {
    cpu.exception(CoprocessorError);
}

pub(super) fn op_swc2(cpu: &mut Cpu, _instruction: Instruction) {
    cpu.exception(CoprocessorError);
}

pub(super) fn op_swc3(cpu: &mut Cpu, _instruction: Instruction) {
    cpu.exception(CoprocessorError);
}

pub(super) fn op_syscall(cpu: &mut Cpu, _:Instruction) {
    cpu.exception(Exception::SysCall);
}