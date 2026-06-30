use vm::Vm;
pub enum OpCode {
    BR = 0, // branch
    ADD,    // add
    LD,     // load
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
pub fn execute_program(vm: &mut Vm) {
    while vm.registers.pc < MEMORY_SIZE {
        //read the instruction
        let instruction = vm.read_memory(vm.registers.pc);
        //increment pc
        vm.registers.pc += 1;
        // extract the opcode then execute the instruction
        instruction::execute_instruction(instruction, vm);
    }
}
pub fn execute_instruction(instr: u16, vm: &mut Vm) {
    //extract the opcode from the instruction
    let opcode = get_op_code(&instr);
    match opcode {
        Some(Opcode::ADD) => add(instr, vm),
        Some(Opcode::AND) => and(instr, vm),
        Some(Opcode::NOT) => not(instr, vm),
        Some(Opcode::BR) => br(instr, vm),
        Some(Opcode::JMP) => jmp(instr, vm),
        Some(Opcode::JSR) => jsr(instr, vm),
        Some(Opcode::LD) => ld(instr, vm),
        Some(Opcode::LDI) => ldi(instr, vm),
        Some(Opcode::LDR) => ld(instr, vm),
        Some(Opcode::LEA) => lea(instr, vm),
        Some(OpCode::ST) => st(instr, vm),
        Some(OpCode::STI) => sti(instr, vm),
        Some(OpCode::STR) => str(instr, vm),
        Some(OpCode::TRAP) => trap(instr, vm),
        _ => {}
    }
}
pub fn read_memory(&mut self, address: u16) -> u16 {
    self.memory[address as usize]
}
