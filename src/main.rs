mod hardware;
use byteorder::{BigEndian, ReadBytesExt};
use hardware::vm::VM;

use std::fs::File;

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
}
