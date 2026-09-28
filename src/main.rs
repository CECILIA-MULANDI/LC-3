mod hardware;
use byteorder::{BigEndian, ReadBytesExt};
use hardware::instruction::execute_program;
use hardware::vm::VM;
use std::fs::File;
use termios::*;

fn main() {
    let mut vm = VM::new();
    let mut f = File::open("program.obj").expect("could not open file");
    let base_address = f.read_u16::<BigEndian>().expect("error");
    let mut address = base_address as usize;
    loop {
        match f.read_u16::<BigEndian>() {
            Ok(instruction) => {
                vm.write_memory(address, instruction);
                address += 1;
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::UnexpectedEof {
                    println!("OK")
                } else {
                    panic!("Error reading file: {}", e);
                }
                break;
            }
        }
    }
    // Raw mode goes: If
    // program.obj is missing, the expect above panics; doing that while the
    // terminal is already in raw mode would leave the shell broken for a
    // failure that had nothing to do with the terminal.
    //
    // Two behaviours are in the way, and both are single bits in c_lflag:
    //   ICANON  the terminal buffers a whole line and releases it on Enter
    //   ECHO    the terminal prints what you type
    // A game needs each keypress immediately and needs to own the screen,
    // so both get cleared.
    // from_fd fails with "Inappropriate ioctl for device" when stdin is not
    // a terminal, which happens whenever input is piped or redirected. That
    // is not an error, there is just no terminal to configure, so skip raw
    // mode and record that there is nothing to restore afterwards.
    let stdin_fd = 0;
    let original = match Termios::from_fd(stdin_fd) {
        Ok(mut termios) => {
            // Termios is Copy, so this is an independent snapshot to restore
            // from and termios stays usable below. No clone needed.
            let original = termios;
            // OR the two flags into one mask, NOT it so it is zeros exactly
            // where they sit and ones everywhere else, then AND to clear only
            // those two bits and leave every other flag alone. The standard
            // clear-bits idiom.
            termios.c_lflag &= !(ICANON | ECHO);
            // TCSANOW: apply immediately rather than waiting for pending output.
            tcsetattr(stdin_fd, TCSANOW, &termios).expect("could not enter raw mode");
            Some(original)
        }
        Err(_) => None,
    };

    execute_program(&mut vm);

    // Put the terminal back, if there was one to configure in the first place.
    //
    // NOTE: this is currently unreachable for any program that ends in HALT,
    // because trap 0x25 calls process::exit(0), which terminates the process
    // without unwinding or returning here. restore_terminal() in the trap
    // handler covers that path. This is correct for other exit paths and is
    // what we want once HALT stops killing the process.
    if let Some(original) = original {
        tcsetattr(stdin_fd, TCSANOW, &original).expect("could not restore terminal settings");
    }
}
