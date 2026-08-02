use crate::hardware::vm::{MEMORY_SIZE, VM};
use std::io::{self, Write};
use std::process;

pub enum OpCode {
    BR = 0, // branch
    ADD,    // add
    LD,     // load
    ST,     // store
    JSR,    // jump register
    AND,    // bitwise and
    LDR,    // load register
    STR,    // store register
    RTI,    // unused
    NOT,    // bitwise not
    LDI,    // load indirect
    STI,    // store indirect
    JMP,    // jump
    RES,    // reserved (unused)
    LEA,    // load effective address
    TRAP,   // execute trap
}

pub fn get_op_code(instruction: &u16) -> Option<OpCode> {
    match instruction >> 12 {
        0 => Some(OpCode::BR),
        1 => Some(OpCode::ADD),
        2 => Some(OpCode::LD),
        3 => Some(OpCode::ST),
        4 => Some(OpCode::JSR),
        5 => Some(OpCode::AND),
        6 => Some(OpCode::LDR),
        7 => Some(OpCode::STR),
        8 => Some(OpCode::RTI),
        9 => Some(OpCode::NOT),
        10 => Some(OpCode::LDI),
        11 => Some(OpCode::STI),
        12 => Some(OpCode::JMP),
        13 => Some(OpCode::RES),
        14 => Some(OpCode::LEA),
        15 => Some(OpCode::TRAP),
        _ => None,
    }
}

fn sign_extend(mut x: u16, bit_count: u8) -> u16 {
    // is the top bit of the original a 1?
    if (x >> (bit_count - 1)) & 1 != 0 {
        // yes -> fill the new positions with 1s
        x |= 0xFFFF << bit_count;
    }
    // otherwise leave as-is (already zeros above)
    x
}

/// LEA: DR = PC + sign_extend(PCoffset9)
///
/// bit position:  15 14 13 12 | 11 10 9 | 8 7 6 5 4 3 2 1 0
///                └── 1110 ──┘ └── DR ──┘ └── PCoffset9 ────┘
///                  opcode      3 bits     9 bits (signed)
pub fn lea(instruction: u16, vm: &mut VM) {
    let dr = (instruction >> 9) & 0x7;
    let pc_offset = sign_extend(instruction & 0x1FF, 9);
    let addr = vm.registers.pc.wrapping_add(pc_offset);
    vm.registers.update(dr, addr);
    vm.registers.update_r_cond_register(dr);
}

/// ADD has two modes
///  1.Register mode(bit 5 = 0)
/// so we can have DR = SR1 + SR2
/// 2. Immediate mode(bit 5 = 1)
pub fn add(instruction: u16, vm: &mut VM) {
    let dr = (instruction >> 9) & 0x7;
    let sr1 = (instruction >> 6) & 0x7;
    let mod_flag = (instruction >> 5) & 0x1;
    let value = if mod_flag == 1 {
        let sign_extended = sign_extend(instruction & 0x1F, 5);
        vm.registers.get(sr1).wrapping_add(sign_extended)
    } else {
        let sr2 = (instruction) & 0x7;
        vm.registers.get(sr1).wrapping_add(vm.registers.get(sr2))
    };
    vm.registers.update(dr, value);
    vm.registers.update_r_cond_register(dr);
}
/// AND has two modes
/// So we need to take two numbers, do a bitwise AND
/// then store that result in some register
///  SO:: if both bits are 1 the result will always be 1
/// Otherwise: it will always be 0
///  MODES: Register mode (bit 5 = 0)
///         Immediate mode(bit 5 = 1)
pub fn and(instruction: u16, vm: &mut VM) {
    let dr = (instruction >> 9) & 0x7;
    let sr1 = (instruction >> 6) & 0x7;
    let mod_flag = (instruction >> 5) & 0x1;
    let value = if mod_flag == 1 {
        let imm5 = sign_extend(instruction & 0x1F, 5);
        vm.registers.get(sr1) & imm5
    } else {
        let sr2 = instruction & 0x7;
        vm.registers.get(sr1) & vm.registers.get(sr2)
    };
    vm.registers.update(dr, value);
    vm.registers.update_r_cond_register(dr);
}
/// ldi - Load Indirect.
/// Reads memory cell to get a pointer
/// then reads that address to get the value
/// So we have two memory reads
/// bit position:  15 14 13 12 | 11 10 9 | 8 7 6 5 4 3 2 1 0
///                └── 1110 ──┘ └── DR ──┘ └── PCoffset9 ────┘
///                  opcode      3 bits     9 bits (signed)
/// pointer_addr = PC + sign_extend(PCoffset9)
/// pointer = memory[pointer_addr]
/// value = memory[pointer]
/// DR = value
/// update condition flags for DR
pub fn ldi(instruction: u16, vm: &mut VM) {
    let dr = (instruction >> 9) & 0x7;
    let pc_offset = sign_extend(instruction & 0x1FF, 9);
    let addr = vm.registers.pc.wrapping_add(pc_offset);
    let pointer = vm.read_memory(addr);
    let value = vm.read_memory(pointer);
    vm.registers.update(dr, value);
    vm.registers.update_r_cond_register(dr);
}

/// TRAP: dispatch to a system-call service by trap vector.
///
/// bit position:  15 14 13 12 | 11 10 9 8 | 7 6 5 4 3 2 1 0
///                └── 1111 ──┘ └─ unused ─┘└─ trap vector ─┘
///                  opcode       4 bits         8 bits
pub fn trap(instruction: u16, vm: &mut VM) {
    match instruction & 0xFF {
        0x21 => {
            // OUT: print single char from R0
            print!("{}", (vm.registers.r0 as u8) as char);
            io::stdout().flush().expect("failed to flush")
        }
        0x22 => {
            // PUTS: print null-terminated string starting at R0
            let mut index = vm.registers.r0;
            let mut c = vm.read_memory(index);
            while c != 0x0000 {
                print!("{}", (c as u8) as char);
                index = index.wrapping_add(1);
                c = vm.read_memory(index);
            }
            io::stdout().flush().expect("failed to flush");
        }
        0x25 => {
            // HALT
            println!("HALT detected");
            io::stdout().flush().expect("failed to flush");
            process::exit(0);
        }
        other => {
            println!("unimplemented trap: {other}");
            process::exit(1);
        }
    }
}

pub fn execute_instruction(instr: u16, vm: &mut VM) {
    let opcode = get_op_code(&instr);
    match opcode {
        Some(OpCode::ADD) => add(instr, vm),
        Some(OpCode::LEA) => lea(instr, vm),
        Some(OpCode::TRAP) => trap(instr, vm),
        Some(OpCode::LDI) => ldi(instr, vm),
        Some(OpCode::AND) => and(instr, vm),
        // other opcodes will be added as the tutorial progresses
        _ => {}
    }
}
//fetch->save->increment->execute saved copy

pub fn execute_program(vm: &mut VM) {
    while (vm.registers.pc as usize) < MEMORY_SIZE {
        //fetch & save memory[pc]
        let instr = vm.read_memory(vm.registers.pc);
        //increment pc count
        vm.registers.pc = vm.registers.pc.wrapping_add(1);
        //execute the saved copy
        execute_instruction(instr, vm);
    }
}
