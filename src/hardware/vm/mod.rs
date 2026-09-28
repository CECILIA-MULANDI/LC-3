use crate::hardware::register::Registers;
use std::io::Read;
pub const MEMORY_SIZE: usize = u16::MAX as usize + 1; // 65536
pub struct VM {
    pub memory: [u16; MEMORY_SIZE],
    pub registers: Registers,
}
// Memory-mapped device registers. These two addresses are not storage:
// reading KBSR asks the real keyboard whether a key is waiting, which is
// how the LC-3 does input without owning a single input instruction. A
// program reads them with LDI, like any other address.
//
// KBSR  bit 15 set means a key is ready, 0x0000 means nothing yet
// KBDR  the character itself, in the low 8 bits
//
// The gap at 0xFE01 is inherited from byte-addressed relatives of the
// LC-3, not derived from anything here.
pub enum MAGICADDR {
    KBSR = 0xFE00,
    KBDR = 0xFE02,
}
impl VM {
    pub fn new() -> VM {
        VM {
            memory: [0; MEMORY_SIZE],
            registers: Registers::new(),
        }
    }
    pub fn write_memory(&mut self, address: usize, value: u16) {
        self.memory[address] = value;
    }
    // Reading KBSR refreshes both device cells before the array access, so
    // the value returned is current. Every other address is a plain array
    // read and the `if` simply does not fire.
    //
    // No `else` here on purpose: the keyboard check is not an alternative
    // to reading memory, it is making memory correct before it is read.
    //
    // KBDR needs no case of its own, which is exactly why a program that
    // reads 0xFE02 without first reading 0xFE00 gets a stale character:
    // nothing refreshes KBDR on its own.
    pub fn read_memory(&mut self, address: u16) -> u16 {
        if address == MAGICADDR::KBSR as u16 {
            self.handle_keyboard();
        }
        self.memory[address as usize]
    }

    // Ask stdin for one byte and publish the answer into the two cells.
    //
    // read_exact BLOCKS, so it waits for a keypress rather than reporting
    // "nothing there". That means the LC-3 polling loop never really spins:
    // its first read of KBSR blocks until you type, then reports ready.
    // Not faithful to real hardware, where polling genuinely burns cycles,
    // but correct for any program that polls.
    //

    fn handle_keyboard(&mut self) {
        let mut buffer = [0u8; 1];
        std::io::stdin()
            .read_exact(&mut buffer)
            .expect("failed to read from stdin");
        if buffer[0] != 0 {
            // 1 << 15 is 0x8000, the ready flag. It sits at bit 15 so that
            // a program can test it with a single BRzp: bit 15 set makes
            // the value negative in two's complement, so the condition
            // flag does the work and no masking is needed.
            self.memory[MAGICADDR::KBSR as usize] = 1 << 15;
            self.memory[MAGICADDR::KBDR as usize] = buffer[0] as u16;
        } else {
            self.memory[MAGICADDR::KBSR as usize] = 0;
        }
    }
}
