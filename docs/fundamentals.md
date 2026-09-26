# What the tutorial assumes you already know

When I started going through the [tutorial](https://www.rodrigoaraujo.me/posts/lets-build-an-lc-3-virtual-machine/), I saw that it explains the LC-3 well. But I thought that it barely explained the layer underneath the LC-3: how a fixed number of bits represents a number, and how you get a field out of the middle of a word. Every place I got stuck turned out to be at that layer rather than at the architecture layer.

This file is my attempt to fill those gaps.
Please **NOTE**: It is not a substitute for the tutorial, you can use it as a prerequisite reading the tutorial does not name or a companion as you go through the tutorial.

## Two's complement

`sign_extend` is the first function in the tutorial that does something non-obvious, and it is impossible to understand until you accept one idea:

> A bit pattern has no value until you say how wide it is.

That sounds like a technicality. It is the whole thing.

### The problem

A register holds 16 bits. There are 65,536 distinct patterns, and that is all the machine has. If you want negative numbers, they have to come out of that same budget. There is no sign flag hiding anywhere, no extra bit. Some of those 65,536 patterns have to _mean_ negative.

### The obvious scheme, and why it fails

Spend the top bit as a sign. `0` means positive, `1` means negative, and the bits below it carry the magnitude. To negate a number you flip the top bit, and nothing else moves.

```
0 101  =  +5        sign 0, magnitude 5
1 101  =  -5        sign 1, magnitude 5
```

This is called sign-magnitude, and it is the first thing everyone proposes, because it is how we write numbers on paper: `-5` is `5` with a mark stuck on the front. Hardware does not use it. It fails in two ways, and both come from the same root, which is that the top bit here is a _flag_ rather than a number.

**It produces two zeros.** `0000` is `+0` and `1000` is `-0`. Both have to mean zero, because integers have no negative zero, so 16 patterns carry only 15 distinct values and one pattern is dead.

The wasted pattern is not the real cost. The real cost is that every zero test now needs two cases. Compare what your own code can get away with against what it would need:

```rust
if self.get(r) == 0 {                              // two's complement
if self.get(r) == 0 || self.get(r) == 0x8000 {     // sign-magnitude
```

That second form would be required at every zero comparison in the machine, in hardware just as much as in this emulator.

**It breaks addition.** This is the serious one. Add `+5` and `-3`, which should give `+2`:

```
   0101     (+5)
 + 1011     (-3, sign-magnitude)
 ------
   10000  →  0000  =  +0     wrong, and not wrong by a predictable amount
```

The adder gave the top bit its usual weight of 8, because a bit sitting in that column is worth 8. But in sign-magnitude that bit does not mean 8, it means "negate everything below me". An adder adds weights. It cannot carry out an instruction.

Rescuing the scheme means the hardware has to inspect both sign bits before doing anything: add the magnitudes when the signs agree, and when they disagree, compare the two magnitudes, subtract the smaller from the larger, and take the sign of the larger. That is a comparator and a subtractor bolted around the adder plus control logic to choose between them, and the comparison has to finish before the subtraction can start, so it is slower as well as bigger.

Sign-magnitude is not a strawman, incidentally. IEEE floating point does use a sign bit exactly this way, and it pays both bills in full: `+0.0` and `-0.0` are genuinely distinct values in every floating-point unit, and float addition really does carry that compare-and-branch logic.

### The scheme that is actually used

Two's complement makes one change. The top bit still marks negatives, but instead of being a flag it carries a _negative weight_.

At 4 bits the weights are normally 8, 4, 2, 1. In two's complement they are **-8**, 4, 2, 1. Nothing else changes. You read the number by adding up the weights of the bits that are set, exactly as before:

```
1101  =  -8 + 4 + 0 + 1  =  -3
0101  =   0 + 4 + 0 + 1  =   5
1000  =  -8 + 0 + 0 + 0  =  -8
1111  =  -8 + 4 + 2 + 1  =  -1
```

That is the entire rule. You do not need "invert the bits and add one" to read a value, only to negate one.

All sixteen 4-bit patterns:

| Bits   | Unsigned | Signed |
| ------ | -------- | ------ |
| `0000` | 0        | 0      |
| `0001` | 1        | 1      |
| `0010` | 2        | 2      |
| `0011` | 3        | 3      |
| `0100` | 4        | 4      |
| `0101` | 5        | 5      |
| `0110` | 6        | 6      |
| `0111` | 7        | 7      |
| `1000` | 8        | -8     |
| `1001` | 9        | -7     |
| `1010` | 10       | -6     |
| `1011` | 11       | -5     |
| `1100` | 12       | -4     |
| `1101` | 13       | -3     |
| `1110` | 14       | -2     |
| `1111` | 15       | -1     |

Read the signed column from the bottom up and you can see what the scheme really is: counting down from zero wraps around to the top of the unsigned range. `0` minus `1` is `1111`. The negatives are the patterns you land on by counting backwards past zero.

### Why hardware likes it

Because addition needs no special cases at all. Here is the sum sign-magnitude got wrong, `+5` plus `-3`, with `-3` now encoded the two's complement way as `1101`:

```
   0101     (+5)
 + 1101     (-3, two's complement)
 ------
   10010  →  0010  =  +2     correct
```

One bit of the encoding changed, `1011` became `1101`, and the same adder now produces the right answer. The result needed five bits and the register has four, so the top bit falls off the end and is discarded. Nothing inspected a sign. Nothing branched.

This is why the LC-3 has one `ADD` instruction rather than a signed and an unsigned one, and it is why `wrapping_add` is the right Rust operation: truncation is not a bug being tolerated, it is the mechanism working.

### The part that matters for sign extension

Look at what happens when the same bits are read at a different width. Take `11101`.

At 5 bits the weights are -16, 8, 4, 2, 1:

```
11101  =  -16 + 8 + 4 + 0 + 1  =  -3
```

At 16 bits that same pattern is `0000000000011101`, and the weights are -32768, 16384, ... 8, 4, 2, 1. The leading zeros contribute nothing, and the top bit of the _word_ is 0, not 1:

```
0000000000011101  =  16 + 8 + 4 + 0 + 1  =  29
```

Same bits. `-3` and `+29`. Nothing was corrupted. The negative weight moved from position 4 to position 15, and position 4's weight became an ordinary `+16`.

This is why widening a signed field cannot be a no-op, and it is the hidden prerequisite the tutorial never states.

## Sign extension

Now the function reads as the obvious consequence of the above.

The LC-3 packs a 5-bit signed immediate (`imm5`) into an instruction, but arithmetic happens in 16-bit registers. Copying those 5 bits into a register unchanged would turn `-3` into `+29`. The fix is to widen it so the value survives: fill every bit above the field with copies of the field's sign bit.

```rust
fn sign_extend(mut x: u16, bit_count: u8) -> u16 {
    if (x >> (bit_count - 1)) & 1 != 0 {
        x |= 0xFFFF << bit_count;
    }
    x
}
```

Traced with `x = 0b11101` and `bit_count = 5`:

1. `x >> 4` shifts the sign bit down to position 0, giving `0b1`. Mask with `& 1` to discard everything else. The result is `1`, so the field is negative.
2. `0xFFFF << 5` is `0b1111111111100000`: ones everywhere above the field, zeros in the field's own five positions.
3. OR that into `x`. The zeros in the low five positions guarantee the field's own bits pass through untouched, and the ones above it become the extension.

```
  0000000000011101     x
| 1111111111100000     0xFFFF << 5
  ----------------
  1111111111111101     result
```

Check it against the weights: all ones from position 15 down to position 2, then `0`, then `1`. That is `-1` with the `2` bit cleared, which is `-3`. The value survived the widening.

There is no `else` branch because none is needed. If the sign bit is `0` the high bits of `x` are already zero, and zero-filling _is_ the correct extension for a positive value.

The same function handles `PCoffset9` with `bit_count = 9`, and every other signed field in the instruction set. Nothing about it is specific to `imm5`.

## Getting a field out of a word

An instruction is one 16-bit word and it has to carry everything: which operation, which registers, and any constant. There are no separators in it. Nothing in the bits says "the register number ends here". The layout is fixed per instruction and you are expected to know it.

Reading the tutorial, each instruction arrives with its own set of shifts and masks, and they look like a different incantation every time. They are not. There is one move, repeated:

> Shift the field down to bit 0, then mask off as many bits as the field is wide.

Two steps, in that order. The shift moves the field's lowest bit to position 0 and throws away everything below it. The mask throws away everything above it. What is left is the field, as a small number.

The mask for an `n`-bit field is `n` ones, which is `2^n - 1`:

| Field width | Binary      | Mask    | Used for                |
| ----------- | ----------- | ------- | ----------------------- |
| 1 bit       | `1`         | `0x1`   | mode bit (bit 5)        |
| 3 bits      | `111`       | `0x7`   | a register number       |
| 5 bits      | `11111`     | `0x1F`  | `imm5`                  |
| 8 bits      | `11111111`  | `0xFF`  | trap vector             |
| 9 bits      | `111111111` | `0x1FF` | `PCoffset9`             |

`0x7` is not a defensive bound. There are 8 registers, a register number is 3 bits, and a 3-bit field cannot hold anything but 0 to 7. The mask is the field's width expressed as a number.

### Traced on a real instruction

`ADD R2, R3, #-3` in immediate mode assembles to `0x14FD`. Split into its fields:

```
 0001   010   011   1   11101
 ────   ───   ───   ─   ─────
opcode   DR   SR1  mode  imm5
  15-12  11-9  8-6   5    4-0
```

Now pull each one out with the recipe:

```rust
instruction >> 12            // 0x1  = ADD
(instruction >> 9) & 0x7     // 0b010 = 2, so DR is R2
(instruction >> 6) & 0x7     // 0b011 = 3, so SR1 is R3
(instruction >> 5) & 0x1     // 0b1   = immediate mode
instruction & 0x1F           // 0b11101, then sign_extend to -3
```

Two details worth noticing. `imm5` needs no shift, because it already sits at the bottom of the word, so the recipe collapses to just the mask. And the opcode needs no mask, because shifting a `u16` right by 12 leaves only 4 bits with nothing above them to discard. That is why `instruction >> 12` appears bare while everything else is a shift-and-mask pair.

The five-bit result `11101` is then handed to `sign_extend`, which is where the previous section picks up. Extraction gets you the bits. Sign extension gets you the value.

## Condition flags are one-hot, and that is a decision

After any instruction that writes a register, the machine records whether the result was negative, zero or positive, in `cond`. `BR` then branches if the last result matched what it is looking for.

The encoding looks arbitrary at first:

```rust
enum ConditionFlag {
    POS = 1 << 0,  // 001
    ZRO = 1 << 1,  // 010
    NEG = 1 << 2,  // 100
}
```

Three states, so why not store 0, 1 and 2? Because of what `BR` has to do. A `BR` instruction does not test one condition, it tests a _set_ of them. `BRnz` means "branch if the result was negative or zero", and the instruction carries three independent bits for that, bit 11 for n, bit 10 for z, bit 9 for p.

If `cond` held 0, 1 or 2, comparing it against those three bits would mean decoding the number and then checking each bit separately. Instead, each state gets its own bit position, and the positions are chosen to line up with the instruction's own n/z/p field:

```
instruction >> 9 & 0x7    →   bit 2 = n   bit 1 = z   bit 0 = p
ConditionFlag             →   NEG = 100   ZRO = 010   POS = 001
```

Same three positions, same order. So the test becomes a set intersection, and set intersection on bits is a single AND:

```rust
if flags & vm.registers.cond != 0 {
```

Traced for `BRnz` after a negative result. The instruction's field is `110`, `cond` holds `NEG` which is `100`:

```
  110     what the instruction will accept (n, z)
& 100     what actually happened (negative)
  ---
  100     non-zero, so branch
```

After a positive result instead, `cond` holds `001`:

```
  110
& 001
  ---
  000     zero, so fall through
```

Two consequences fall out of this that are worth having noticed:

Exactly one bit of `cond` is ever set, because a value is exactly one of negative, zero or positive. The instruction's field has no such restriction. All three bits set, `BRnzp`, always intersects whatever `cond` holds, which is how you write an unconditional branch without needing a separate instruction for it. No bits set never branches at all.

Every handler that writes a register must call `update_r_cond_register(dr)` afterwards. Forget it in one handler and `BR` does not fail loudly, it silently tests the result of some earlier instruction. That is the kind of bug that looks like a broken branch when the branch is fine.

One last link back: `update_r_cond_register` decides negativity with `(self.get(r) >> 15) != 0`, testing the top bit of the word. That works because of two's complement. The top bit of a 16-bit word _is_ the sign.

## Memory is word-addressed

On the machine you are writing this on, an address names a byte. On the LC-3, an address names a 16-bit word. There is no way to address half of one.

```rust
pub const MEMORY_SIZE: usize = u16::MAX as usize + 1;  // 65,536
memory: [u16; MEMORY_SIZE]
```

The array type is the whole idea. A 16-bit address can name 65,536 distinct locations, and each location holds one `u16`. The address space is exactly saturated: there is no address you cannot form and no slot you cannot reach. `u16::MAX + 1` rather than `u16::MAX` because addresses start at zero.

What this changes in practice:

**Address arithmetic counts words.** In `PUTS`, `index.wrapping_add(1)` advances one character, because a character occupies an entire word.

**Which means LC-3 strings waste half their space.** One character per 16-bit word, high 8 bits unused, which is why printing casts through `u8` first with `(c as u8) as char`. It is also the reason `PUTSP` exists as a separate trap: it packs two characters per word for programs that care.

**Alignment is not a concept here.** Every access is one word at one address. There is no unaligned read to get wrong.

## The PC is incremented before the handler runs

The fetch loop does three things in a fixed order:

```rust
let instr = vm.read_memory(vm.registers.pc);          // fetch
vm.registers.pc = vm.registers.pc.wrapping_add(1);    // advance
execute_instruction(instr, vm);                       // execute
```

The consequence is easy to skim past and it affects every PC-relative instruction. By the time a handler reads `vm.registers.pc`, it holds the address of the _next_ instruction, not the address of the instruction being executed. So every PC-relative offset is measured from the following instruction, and an offset of 0 means "the next instruction".

Traced, with `LEA R0, #2` sitting at `0x3000`:

```
fetch  memory[0x3000]      pc = 0x3000
advance                    pc = 0x3001
execute  addr = pc + 2  →  0x3003
```

`R0` ends up holding `0x3003`, which is three words past the `LEA` itself but two past the instruction after it.

This is not an artifact of how the loop happens to be written. It is the LC-3 spec, and it mirrors real hardware, where the PC is incremented during the fetch cycle before the execute cycle begins. Assemblers compute their offsets on that assumption, so incrementing after execute instead would put every offset in every assembled program off by one.

It also explains why the instruction is copied into `instr` before the increment rather than re-read later. `BR` and, later, `JMP` and `JSR` all write to `pc` themselves. Once a handler has run, `pc` may have nothing to do with where the current instruction lived.

## Object files are big-endian

A `.obj` file is a sequence of 16-bit words, but files are sequences of bytes, so each word has to be split into two. There are two possible orders and the choice is not observable from the file itself.

Big-endian puts the most significant byte first. Little-endian puts the least significant byte first. LC-3 object files are big-endian. x86 and ARM are little-endian. The mismatch is real and it bites immediately, on the very first word.

The origin word `0x3000` is stored in the file as the bytes `30 00`. Read those two bytes into a `u16` assuming the host's own order, and the first byte lands in the low half:

```
file bytes:        30 00
big-endian:        0x3000   =  12288   correct
little-endian:     0x0030   =     48   wrong
```

The program would load at address 48 instead of 0x3000, `PC_START` would point at empty memory, and nothing would run.

```rust
f.read_u16::<BigEndian>()
```

The type parameter names the order _in the file_, not the order of the machine doing the reading. `byteorder` reads two bytes and assembles them in the declared order, swapping if the host disagrees. The same line is correct on a big-endian host, where it just happens to swap nothing.

Two more things about the format, both of which the loader in `main.rs` depends on:

The first word is not an instruction. It is the address to start loading at, usually `0x3000`, which is why `PC_START` is also `0x3000`. Every word after it is loaded sequentially from there.

There is no length header and no end marker, so running out of file is the only signal that loading is finished. That is why `UnexpectedEof` is matched as the normal, successful end of the loop rather than treated as an error. Any _other_ `io::Error` is a real failure and does panic.
