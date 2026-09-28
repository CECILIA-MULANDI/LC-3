use crate::hardware::instruction;
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
// NOT - bitwise complement
// DR = !SR, every one of the 16 bits flipped
//
// bit position:  15 14 13 12 | 11 10 9 | 8 7 6 | 5 | 4 3 2 1 0
//                └── 1001 ──┘ └─ DR ──┘ └─ SR ─┘ 1   1 1 1 1 1
//                  opcode      3 bits    3 bits   └─ always 1s ─┘
//
// Bits 5..0 are hardwired to 1 by the spec and carry no information,
// so there is nothing to decode down there.
//
// Worth remembering: NOT 0x0000 gives 0xFFFF, which is -1, so the flag
// afterwards is NEG and not ZRO. DR and SR are allowed to be the same
// register, which flips it in place and needs no special case here.
pub fn not(instruction: u16, vm: &mut VM) {
    let dr = (instruction >> 9) & 0x7;
    let sr1 = (instruction >> 6) & 0x7;
    let value = vm.registers.get(sr1);
    let result = !value;
    vm.registers.update(dr, result);
    vm.registers.update_r_cond_register(dr);
}

// LD - Load
// from memory to register
// DR = memory[PC + sign_extend(PCoffset9)]
//
// bit position:  15 14 13 12 | 11 10 9 | 8 7 6 5 4 3 2 1 0
//                └── 0010 ──┘ └─ DR ──┘ └── PCoffset9 ────┘
//                  opcode      3 bits    9 bits (signed)
//
// This is LDI with one dereference removed: the cell at PC + offset
// holds the value itself, not a pointer to it. The offset is signed so
// it can reach backwards, and it is relative to the NEXT instruction,
// because the fetch loop has already advanced pc before we run.
pub fn ld(instruction: u16, vm: &mut VM) {
    let dr = (instruction >> 9) & 0x7;
    let pc_offset = sign_extend(instruction & 0x1FF, 9);
    let addr = vm.registers.pc.wrapping_add(pc_offset);
    let value = vm.read_memory(addr);
    vm.registers.update(dr, value);
    vm.registers.update_r_cond_register(dr);
}

// ST - Store
// from register to memory
// memory[PC + sign_extend(PCoffset9)] = SR
//
// bit position:  15 14 13 12 | 11 10 9 | 8 7 6 5 4 3 2 1 0
//                └── 0011 ──┘ └─ SR ──┘ └── PCoffset9 ────┘
//                  opcode      3 bits    9 bits (signed)
//
// Identical layout to LD. The only differences are the opcode and that
// the register field is now a source rather than a destination, so the
// value moves the other way.
//
// No flag update: the flags describe the last value written to a
// register, and this writes memory while leaving every register alone.
// Calling update_r_cond_register here would set flags from a stale
// value and a later BR would branch on it.
//
// Note write_memory takes usize while read_memory takes u16, hence the
// cast.
pub fn st(instruction: u16, vm: &mut VM) {
    let sr = (instruction >> 9) & 0x7;
    let pc_offset = sign_extend(instruction & 0x1FF, 9);
    let addr = vm.registers.pc.wrapping_add(pc_offset);
    let value = vm.registers.get(sr);
    vm.write_memory(addr as usize, value);
}
// STI - Store Indirect
// memory[memory[PC + offset]] = SR
//
// bit position:  15 14 13 12 | 11 10 9 | 8 7 6 5 4 3 2 1 0
//                └── 1011 ──┘ └─ SR ──┘ └── PCoffset9 ────┘
//                  opcode      3 bits    9 bits (signed)
//
// Same as ST, but the cell at PC + offset does not receive the value.
// It holds the address that does. So two memory accesses: one read to
// fetch the pointer, one write to the place the pointer names.
pub fn sti(instruction: u16, vm: &mut VM) {
    let sr = (instruction >> 9) & 0x7;
    let pc_offset = sign_extend(instruction & 0x1FF, 9);
    let addr = vm.registers.pc.wrapping_add(pc_offset);
    // read the pointer out of memory
    let pointer = vm.read_memory(addr);
    let value = vm.registers.get(sr);
    // write at the address the pointer gave us, not at addr
    vm.write_memory(pointer as usize, value);
    // no flag update: memory changed, registers did not
}

// LDR - Load Register (base + offset)
// DR = memory[BaseR + sign_extend(offset6)]
//
// bit position:  15 14 13 12 | 11 10 9 | 8 7 6 | 5 4 3 2 1 0
//                └── 0110 ──┘ └─ DR ──┘ └Base R┘ └─ offset6 ─┘
//                  opcode      3 bits    3 bits   6 bits (signed)
//
// The address comes from a register instead of the PC, so the program
// picks the base at runtime. That is how you walk an array or reach a
// field inside a struct: point BaseR at the start, offset to the member.
// The offset is only 6 bits, so the mask is 0x3F and sign_extend gets 6.
pub fn ldr(instruction: u16, vm: &mut VM) {
    let dr = (instruction >> 9) & 0x7;
    let base_r = (instruction >> 6) & 0x7;
    let offset = sign_extend(instruction & 0x3F, 6);
    let addr = vm.registers.get(base_r).wrapping_add(offset);
    let value = vm.read_memory(addr);
    vm.registers.update(dr, value);
    vm.registers.update_r_cond_register(dr);
}

// STR - Store Register (base + offset)
// memory[BaseR + sign_extend(offset6)] = SR
//
// bit position:  15 14 13 12 | 11 10 9 | 8 7 6 | 5 4 3 2 1 0
//                └── 0111 ──┘ └─ SR ──┘ └Base R┘ └─ offset6 ─┘
//                  opcode      3 bits    3 bits   6 bits (signed)
//
// LDR with the value moving the other way. Address arithmetic is
// identical; only the direction changes.
pub fn str(instruction: u16, vm: &mut VM) {
    let sr = (instruction >> 9) & 0x7;
    let base_r = (instruction >> 6) & 0x7;
    let offset = sign_extend(instruction & 0x3F, 6);
    let addr = vm.registers.get(base_r).wrapping_add(offset);
    let value = vm.registers.get(sr);
    vm.write_memory(addr as usize, value);
    // no flag update, same reason as ST and STI
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

/// BR - allows branching just like if/else, while or for loops
/// bit position:  15 14 13 12 | 11 | 10 | 9 | 8 7 6 5 4 3 2 1 0
///                 └── 0000 ──┘  n    z    p  └── PCoffset9 ────┘
///                opcode         ↑    ↑    ↑    9 bits (signed)
///                  └─ flags to test ─┘
/// 1. Look at the instruction's n/z/p bits.
/// 2. Look at what the cond register currently holds
///( which is N, Z, or P from the LAST arithmetic op).
/// 3. If any bit in the instruction's n/z/p matches the currently-set flag → JUMP.
///    PC = PC + sign_extend(PCoffset9)
///  4. Otherwise → do nothing.
/// (PC has already been advanced by the fetch loop, so we just fall through
///  to the next instruction.)
pub fn br(instruction: u16, vm: &mut VM) {
    let flags = (instruction >> 9) & 0x7;
    // the overlap test
    if flags & vm.registers.cond != 0 {
        let pc_offset = sign_extend(instruction & 0x1FF, 9);
        vm.registers.pc = vm.registers.pc.wrapping_add(pc_offset);
    }
}

// JMP - unconditional jump
// PC = BaseR
//
// bit position:  15 14 13 12 | 11 10 9 | 8 7 6 | 5 4 3 2 1 0
//                └── 1100 ──┘  0  0  0  └Base R┘  0 0 0 0 0 0
//                  opcode       unused   3 bits      unused
//
// Absolute, not PC-relative, so it reaches the whole 64K address space
// while BR can only manage PC +/- 256. No arithmetic, so no wrapping_add,
// and no flag update because the PC is not a general-purpose result.
//
// RET is not a separate opcode. It assembles to 0xC1C0, which is this
// instruction with BaseR = 7, so the handler covers it with no special
// case.
pub fn jmp(instruction: u16, vm: &mut VM) {
    let base_r = (instruction >> 6) & 0x7;
    vm.registers.pc = vm.registers.get(base_r);
}

// JSR / JSRR - jump to subroutine
//
// Two things happen, in this order:
//   1. R7 = address of the instruction after this one   (the breadcrumb)
//   2. PC = the subroutine's address                    (the jump)
//
// R7 is not special hardware. The spec designates it by convention as the
// link register: JSR writes it, RET reads it. That is the whole calling
// convention, and it is why RET had nothing useful to return to until now.
//
// The return address needs no computing. execute_program already advanced
// pc before dispatching here, so vm.registers.pc is ALREADY the address
// of the next instruction.
//
// Mode flag is bit 11, not bit 5 like ADD and AND.
//
// bit 11 = 1, JSR, PC-relative:
//   15 14 13 12 | 11 | 10 9 8 7 6 5 4 3 2 1 0
//   └── 0100 ──┘  1   └──── PCoffset11 ──────┘
//     opcode           11 bits (signed), reaches -1024 to +1023
//
// bit 11 = 0, JSRR, absolute from a register:
//   15 14 13 12 | 11 | 10 9 | 8 7 6 | 5 4 3 2 1 0
//   └── 0100 ──┘  0    0  0  └Base R┘  0 0 0 0 0 0
//     opcode           unused  3 bits     unused
//
// The target is worked out BEFORE R7 is written, and that ordering is
// load-bearing. Consider JSRR R7 (0x41C0), which is legal and does occur.
// Write R7 first and the subsequent read of BaseR returns the value just
// written, so the jump lands on the return address, the subroutine never
// runs, and there is no error to notice.
//
// No flag update: R7 is written, but JSR does not set condition codes.
pub fn jsr(instruction: u16, vm: &mut VM) {
    let mode_flag = (instruction >> 11) & 0x1;
    // resolve the target first, while BaseR is still intact
    let target = if mode_flag == 1 {
        let pc_offset = sign_extend(instruction & 0x7FF, 11);
        vm.registers.pc.wrapping_add(pc_offset)
    } else {
        let base_r = (instruction >> 6) & 0x7;
        vm.registers.get(base_r)
    };
    // pc is already pointing at the instruction after this one
    let return_addr = vm.registers.pc;
    vm.registers.update(7, return_addr);
    vm.registers.pc = target;
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
        Some(OpCode::BR) => br(instr, vm),
        Some(OpCode::NOT) => not(instr, vm),
        Some(OpCode::LD) => ld(instr, vm),
        Some(OpCode::ST) => st(instr, vm),
        Some(OpCode::STI) => sti(instr, vm),
        Some(OpCode::LDR) => ldr(instr, vm),
        Some(OpCode::STR) => str(instr, vm),
        Some(OpCode::JSR) => jsr(instr, vm),
        Some(OpCode::JMP) => jmp(instr, vm),
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
