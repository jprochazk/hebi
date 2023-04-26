## Instructions

The language is compiled to a register-based instruction set targetting a custom virtual machine with infinite registers and a pool of constants. The instructions are encoded as a sequence of bytes referred to as the *instruction stream*.

Each instruction begins with a single byte which represents an opcode, followed by zero or more bytes for the instruction operands. The opcode is used as the instruction discriminant, telling the virtual machine *what* to do. The operands contain arbitrary values encoded as integers, and are used to tweak the exact parameters of the operation the virtual machine is being instructed to perform.

For example, the `load_const [0]` instruction (which loads a value from the constant pool at index 0) would be encoded as the 2-byte sequence `05 00` (in hexadecimal).

In some cases, the value of an operand may be larger than 255, in which case it can't be encoded as a single byte anymore. This is referred to as an "operand overflow", the result of which is a widening of the instruction. A wide instruction is encoded with a prefix byte placed before the opcode, which determines the width of *all* of its operands. There are two such prefix bytes: `wide16` (`0x02`), and `wide32` (`0x03`), which represent 16-bit and 32-bit width operands, respectively. The reason that the operand width is not more granular is that it makes encoding and decoding very easy, and in practice most instructions only have one operand, so this encoding scheme is still very space efficient without sacrificing too much throughput.

It is important to note that operands are always encoded in little-endian byte order, even on big-endian systems.

For example, the `wide16.load_const [1000]` instruction would be encoded as the 4-byte sequence `01 03 E8 03`.

This encoding ensures that the instructions take up as little space as possible without giving up the entire 32-bit range for operand values.

## Jump instruction encoding

Because the instruction stream is a *byte* stream, it is aligned to a single byte. This poses some interesting challenges when it comes to encoding jump offsets. Consider the following program:

```python
for i in 0..10:
  print i
```

It's a simple loop that prints integers from `0` to `9`.

Here's what the disassembly for the above program might look like:

```
0  |   load_smi 0
1  |   store r0
   | cond:
2  |   load_smi 10
3  |   cmp_lt r0
4  |   jump_if_false .end
5  |   jump .body
   | latch:
6  |   load_smi 1
7  |   add r0
8  |   store r0
9  |   jump .cond
   | body:
10 |   load r0
11 |   print
12 |   jump .latch
   | end:
13 |   ret
```

When a jump instruction (such as the `jump_if_false .end` on line `4`) is encoded, it must be given a value for the jump offset. The job of a jump instruction is to move the VM's instruction pointer, and the offset determines how much the pointer will move. The problem is that we haven't yet encoded the instructions that come after the jump, so we don't know what the offset should be! There could be any number of instructions of any width (including other jumps) between the jump instruction and its destination.

The solution to this is to encode jump instructions with placeholder values, and keep track of the unfinished ones in a side table. This table can be traversed to patch the jump instructions with their real offsets once we emit all the instructions and find out how far they should actually jump.

But that doesn't solve all of the problems. We still need to know what width to use when encoding the jump instruction, even with a placeholder value. We don't want to shrink or expand the instruction after encoding it, as this could potentially invalidate the offsets of other jump instructions, which would mean having to re-encode all the other jump instructions with their updated offsets. It is possible to do this (the process is called [assembler relaxation](https://eli.thegreenplace.net/2013/01/03/assembler-relaxation) and is common in native assemblers), but we are building an interpreted language, so we want to avoid anything that could greatly increase the latency of our bytecode compiler.

The method chosen by the Hebi VM is:

1. Reserve an entry in the constant pool, which yields an *index*.
1. Encode the jump instruction with the minimum width required to store the *index*.
1. When the jump label is bound, calculate the real offset, and patch the jump instruction:
  * If the offset fits within the same width as the *index*, then store it directly in the jump instruction.
  * Otherwise, store the offset in the reserved constant pool entry, and encode the jump instruction as a `jump_const`.

In practice, most jump offsets do fit within a byte. In case they don't, they will "lifted" into the constant pool and stored as 64-bit, which is hopefully enough address space for the foreseeable future!

## Instruction operands

| name                | operand 0           | operand 0 type        | operand 1   | operand 1 type |
| ------------------- | ------------------- | --------------------- | ----------- | -------------- |
| nop                 |                     |                       |             |                |
| wide16              |                     |                       |             |                |
| wide32              |                     |                       |             |                |
| load_const          | index               | constant index        |             |                |
| load                | register            | register              |             |                |
| store               | register            | register              |             |                |
| load_upvalue        | upvalue             | upvalue index         |             |                |
| store_upvalue       | upvalue             | upvalue index         |             |                |
| load_module_var     | module variable     | module variable index |             |                |
| store_module_var    | module variable     | module variable index |             |                |
| load_global         | global name         | constant index        |             |                |
| store_global        | global name         | constant index        |             |                |
| load_field          | field name          | constant index        |             |                |
| load_field_opt      | field name          | constant index        |             |                |
| store_field         | field name          | constant index        |             |                |
| load_index          | index               | register              |             |                |
| load_index_opt      | index               | register              |             |                |
| store_index         | index               | register              |             |                |
| load_self           |                     |                       |             |                |
| load_super          |                     |                       |             |                |
| load_none           |                     |                       |             |                |
| load_true           |                     |                       |             |                |
| load_false          |                     |                       |             |                |
| load_smi            | value               | integer               |             |                |
| make_fn             | function descriptor | constant index        |             |                |
| upvalue_reg         | source              | register              | destination | upvalue index  |
| upvalue_slot        | source              | upvalue index         | destination | upvalue index  |
| make_class          | class descriptor    | constant index        |             |                |
| jump                | offset              | jump offset           |             |                |
| jump_const          | offset              | constant index        |             |                |
| jump_back           | offset              | jump offset           |             |                |
| jump_back_const     | offset              | constant index        |             |                |
| jump_if_false       | offset              | jump offset           |             |                |
| jump_if_false_const | offset              | constant index        |             |                |
| add                 | rhs                 | register              |             |                |
| sub                 | rhs                 | register              |             |                |
| mul                 | rhs                 | register              |             |                |
| div                 | rhs                 | register              |             |                |
| rem                 | rhs                 | register              |             |                |
| pow                 | rhs                 | register              |             |                |
| inv                 |                     |                       |             |                |
| not                 |                     |                       |             |                |
| cmp_eq              | rhs                 | register              |             |                |
| cmp_ne              | rhs                 | register              |             |                |
| cmp_gt              | rhs                 | register              |             |                |
| cmp_ge              | rhs                 | register              |             |                |
| cmp_lt              | rhs                 | register              |             |                |
| cmp_le              | rhs                 | register              |             |                |
| cmp_type            | rhs                 | register              |             |                |
| contains            | rhs                 | register              |             |                |
| print               |                     |                       |             |                |
| print_n             | start               | register              | count       | integer        |
| call                | function            | register              | args        | integer        |
| import              | path                | constant index        | destination | register       |
| ret                 |                     |                       |             |                |
| suspend             |                     |                       |             |                |

## Instruction descriptions

| name                | description                                                                                           |
| ------------------- | ----------------------------------------------------------------------------------------------------- |
| nop                 | do nothing                                                                                            |
| wide16              | widen the instruction operands to 16 bits                                                             |
| wide32              | widen the instruction operands to 32 bits                                                             |
| load_const          | load a constant into the accumulator                                                                  |
| load                | load a register into the accumulator                                                                  |
| store               | store the accumulator in a register                                                                   |
| load_upvalue        | load an upvalue into the accumulator                                                                  |
| store_upvalue       | store the accumulator in an upvalue                                                                   |
| load_module_var     | load a module variable into the accumulator                                                           |
| store_module_var    | store the accumulator into a module variable                                                          |
| load_global         | load a global into the accumulator                                                                    |
| store_global        | store the accumulator into a global                                                                   |
| load_field          | load a field into the accumulator, panics if the field does not exist                                 |
| load_field_opt      | load a field into the accumulator, yields `none` if the field does not exist                          |
| store_field         | store the accumulator into a field                                                                    |
| load_index          | load an index into the accumulator, panics if the index does not exist                                |
| load_index_opt      | load an index into the accumulator, yields `none` if the index does not exist                         |
| store_index         | store the accumulator into an index                                                                   |
| load_self           | load `self` into the accumulator                                                                      |
| load_super          | load the current super-class into the accumulator                                                     |
| load_none           | load `none` into the accumulator                                                                      |
| load_true           | load boolean `true` into the accumulator                                                              |
| load_false          | load boolean `false` into the accumulator                                                             |
| load_smi            | load an integer value into the accumulator                                                            |
| make_fn             | instantiate a function using a function descriptor                                                    |
| upvalue_reg         | capture a register into an upvalue in the function stored in the accumulator                          |
| upvalue_slot        | capture an upvalue from the parent function into an upvalue in the function stored in the accumulator |
| make_class          | instantiate a class using a class descriptor                                                          |
| jump                | jump forward by `offset` bytes                                                                        |
| jump_const          | jump forward by `offset` bytes (stored in constant pool)                                              |
| jump_back           | jump backward by `offset` bytes                                                                       |
| jump_back_const     | jump backward by `offset` bytes (stored in constant pool)                                             |
| jump_if_false       | jump forward by `offset` bytes if the value in the accumulator is false                               |
| jump_if_false_const | jump forward by `offset` bytes (stored in the constant pool) if the value in the accumulator is false |
| add                 | add a value stored in a register to the accumulator                                                   |
| sub                 | subtract a value stored in a register from the accumulator                                            |
| mul                 | multiply the accumulator by a value stored in a register                                              |
| div                 | divide the accumulator by a value stored in a register                                                |
| rem                 | divide the accumulator by a value stored in a register, and store the remainder in the accumulator    |
| pow                 | raise the accumulator to the power of a value stored in a register                                    |
| inv                 | invert the accumulator                                                                                |
| not                 | negate the accumulator                                                                                |
| cmp_eq              | test if the accumulator is equal to a value stored in a register                                      |
| cmp_ne              | test if the accumulator is not equal to a value stored in a register                                  |
| cmp_gt              | test if the accumulator is greater than a value stored in a register                                  |
| cmp_ge              | test if the accumulator is greater than or equal to a value stored in a register                      |
| cmp_lt              | test if the accumulator is less than a value stored in a register                                     |
| cmp_le              | test if the accumulator is less than or equal to a value stored in a register                         |
| cmp_type            | test if the accumulator is an instance of a class                                                     |
| contains            | test if the accumulator is contained in a value stored in a register                                  |
| print               | print the accumulator                                                                                 |
| print_n             | print `count` values starting at `start`                                                              |
| call                | call a function                                                                                       |
| import              | load the module at `path` into the `destination` register                                             |
| ret                 | return from a function call                                                                           |
| suspend             | stop the dispatch loop                                                                                |


## Calling convention

Hebi expects the following order of parameters for function calls:

```
<function>
arg0
arg1
...
argN
```

Where `<function>` is the function being called, and `arg` are the actual arguments.
Call instructions use a register operand to point to the location of the function on the stack, and a second operand to store the number of arguments the function is being called with.

The actual call operation is simple:
1. Prepare a new call frame
1. Allocate stack space for registers
1. Copy arguments to the new stack space
1. Jump to the function's start

Before the call, the VM checks that the value being called is a function, and that the function is being provided with the correct number of arguments.

For example, consider the following program:

```rust
fn add(a, b)
  return a + b

print add(5, 10)
```

It's a simple function which adds up its two parameters and returns the result. The disassembly for the above code is:

```
function `main` (registers: 3, length: 21, pool: 1)
  make_fn [0] ; <function `sum` descriptor>
  store_module_var #0 ; sum
  load_module_var #0 ; sum
  store r0
  load_smi 5
  store r1
  load_smi 10
  store r2
  call r0, 2
  print
  ret

function `sum` (registers: 3, length: 5, pool: 0)
  load r1
  add r2
  ret
```

This is the actual call:

```
  load_module_var #0 ; sum
  store r0
  load_smi 5
  store r1
  load_smi 10
  store r2
  call r0, 2
```

The function is loaded first

```
  load_module_var #0 ; sum
  store r0
```

Followed by the arguments `5` and `10`

```
  load_smi 5
  store r1
  load_smi 10
  store r2
```

The registers for the function and arguments are allocated before any of them are emitted.
The bytecode emitter ensures these registers are contiguous.

## Resources

- https://v8.dev/docs/ignition
- http://www.lua.org/source/5.1/lopcodes.h.html
