# LC-3

Building an LC-3 virtual machine from scratch as a learning exercise.

Following the tutorial: [Let's Build an LC-3 Virtual Machine](https://www.rodrigoaraujo.me/posts/lets-build-an-lc-3-virtual-machine/) by Rodrigo Araujo.

## What is LC-3?

The Little Computer 3 (LC-3) is an educational computer architecture commonly used to teach assembly programming and computer organization fundamentals.

It is a 16-bit machine with 8 general-purpose registers, a program counter, a condition register, and 65,536 words of memory. Note that memory is **word-addressed**, not byte-addressed: each address holds one 16-bit word, so the address space is exactly saturated by `u16`.

## Status

Six of sixteen opcodes are implemented, plus three trap services. That is enough to run programs that do arithmetic, branch, and print, but not yet enough to run a real LC-3 game.

### Implemented

| Opcode | Modes / notes                                  |
| ------ | ---------------------------------------------- |
| `ADD`  | register and immediate (imm5)                  |
| `AND`  | register and immediate (imm5)                  |
| `BR`   | conditional branch on any combination of N/Z/P |
| `LDI`  | load indirect (double dereference)             |
| `LEA`  | load effective address, PC-relative            |
| `TRAP` | dispatches to the trap services below          |

Trap services: `OUT` (0x21), `PUTS` (0x22), `HALT` (0x25).

### Not yet implemented

`NOT`, `JMP`/`RET`, `JSR`/`JSRR`, `LD`, `LDR`, `ST`, `STI`, `STR`.

Also outstanding: memory-mapped keyboard registers (KBSR/KBDR), the input trap services (`GETC`, `IN`, `PUTSP`), and terminal raw mode. Unimplemented opcodes currently fall through the `_ => {}` arm in `execute_instruction` and are silently ignored.

## Running it

```sh
cargo run
```

The binary reads a file named `program.obj` from the current working directory. The path is hardcoded in `main.rs`, there is no CLI argument yet. A checked-in `program.obj` is present for manual testing.

## Implementation notes

Things that I think are easy to forget and painful to rediscover.

**Instruction format.** The top 4 bits (`instruction >> 12`) are always the opcode. The remaining 12 bits are laid out differently per instruction. Fields are pulled out by shifting the field down to bit 0 and masking: `0x7` for a 3-bit register number, `0x1F` for an imm5, `0x1FF` for a PCoffset9, `0xFF` for a trap vector.

**Sign extension.** Short signed fields (imm5, PCoffset9) must be widened to 16 bits by copying the sign bit leftward, because the value of a two's-complement number depends on its width: `11101` is -3 as 5 bits but +29 as 16. `sign_extend` tests the field's top bit and, if set, ORs in `0xFFFF << bit_count`.

**Condition flags are one-hot.** `POS = 001`, `ZRO = 010`, `NEG = 100`; exactly one is set at a time. Every instruction that writes a register must call `update_r_cond_register(dr)` afterwards. The one-hot encoding is what lets `BR` test its condition with a single `flags & cond != 0`.

**PC is incremented before execute.** The fetch loop reads `memory[pc]`, advances `pc`, and only then runs the handler. So PC-relative offsets are relative to the instruction _after_ the current one, meaning an offset of 0 points at the next instruction. This is per the LC-3 spec, not an accident of the loop.

**Object files are big-endian.** The first `u16` of a `.obj` file is the origin address to load at (normally `0x3000`, matching `PC_START`); every word after that is loaded sequentially from there. LC-3 files are big-endian and x86 is little-endian, so loading goes through `byteorder`'s `read_u16::<BigEndian>()`. Hitting `UnexpectedEof` is the normal, expected end of loading.

**Arithmetic wraps.** Rust panics on integer overflow in debug builds, but the LC-3 expects silent wraparound, so address and arithmetic computation uses `wrapping_add` throughout.

**`HALT` is the only way out.** `execute_program` loops while `(pc as usize) < MEMORY_SIZE`, but `pc` is a `u16` and `MEMORY_SIZE` is 65536, so that condition can never be false. `wrapping_add` rolls `0xFFFF` back to `0` rather than exceeding it. The loop does not terminate on its own; `trap 0x25` calling `process::exit(0)` is what ends the program.

## Testing

There is no automated test suite. Opcodes have been verified by hand with small assembled `.obj` files: a hello-world program exercising `LEA`/`PUTS`/`HALT`, and an arithmetic program exercising `ADD` that prints `AB`.

The tutorial being followed does not test opcodes individually; it implements them one at a time and eventually runs full `.obj` programs. The per-opcode test programs here are additional scaffolding, not part of the tutorial.
