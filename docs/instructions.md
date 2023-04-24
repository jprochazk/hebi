## Instructions

The language is compiled to instructions encoded as a sequence of bytes. Each instruction begins with a single byte which represents an opcode, followed by zero or more bytes for the instruction operands. The opcode is used as the instruction discriminant, and the operands contain arbitrary values encoded as integers.

For example, the `ldac 0` instruction would be encoded as the following byte sequence:

```
[ 0x01, 0x00 ]
```

Some instructions have "wide" variants (prefixed by `w`), which behave the same way, but use wide operands. Wide operands take up more memory, but in return allow encoding larger values.

## Bytecode format

| name             | operand 0           | operand 0 type        | operand 1   | operand 1 type |
| ---------------- | ------------------- | --------------------- | ----------- | -------------- |
| nop              |                     |                       |             |                |
| load_const       |                     | constant index        |             |                |
| load_reg         |                     | register              |             |                |
| store_reg        |                     | register              |             |                |
| load_upvalue     |                     | upvalue index         |             |                |
| store_upvalue    |                     | upvalue index         |             |                |
| load_module_var  |                     | module variable index |             |                |
| store_module_var |                     | module variable index |             |                |
| load_global      | global nam          | constant index        |             |                |
| store_global     | global nam          | constant index        |             |                |
| load_field       | field name          | constant index        |             |                |
| load_field_opt   | field name          | constant index        |             |                |
| store_field      | field name          | constant index        |             |                |
| load_index       | index               | register              |             |                |
| load_index_opt   |                     |                       |             |                |
| store_index      | index               | register              |             |                |
| load_self        |                     |                       |             |                |
| load_super       |                     |                       |             |                |
| load_none        |                     |                       |             |                |
| load_true        |                     |                       |             |                |
| load_false       |                     |                       |             |                |
| load_smi         | value               | integer               |             |                |
| make_fn          | function descriptor | constant index        |             |                |
| upvalue_reg      | source              | register              | destination | upvalue index  |
| upvalue_slot     | source              | upvalue index         | destination | upvalue index  |
| make_class       | class descriptor    |                       |             |                |
| jump             | offset              | byte offset           |             |                |
| loop             | offset              | byte offset           |             |                |
| jump_if_false    | offset              | byte offset           |             |                |
| add              | rhs                 | register              |             |                |
| sub              | rhs                 | register              |             |                |
| mul              | rhs                 | register              |             |                |
| div              | rhs                 | register              |             |                |
| rem              | rhs                 | register              |             |                |
| pow              | rhs                 | register              |             |                |
| inv              | rhs                 | register              |             |                |
| not              | rhs                 | register              |             |                |
| cmp_eq           | rhs                 | register              |             |                |
| cmp_ne           | rhs                 | register              |             |                |
| cmp_gt           | rhs                 | register              |             |                |
| cmp_ge           | rhs                 | register              |             |                |
| cmp_lt           | rhs                 | register              |             |                |
| cmp_le           | rhs                 | register              |             |                |
| is               | rhs                 | register              |             |                |
| in               | rhs                 | register              |             |                |
| print            | start               | register              | count       | integer        |
| call             |                     |                       |             |                |
| return           |                     |                       |             |                |
| yield            |                     |                       |             |                |

## Instruction descriptions

| name             | description                                                                                           |
| ---------------- | ----------------------------------------------------------------------------------------------------- |
| nop              | do nothing                                                                                            |
| load_const       | load a constant into the accumulator                                                                  |
| load_reg         | load a register into the accumulator                                                                  |
| store_reg        | store the accumulator in a register                                                                   |
| load_upvalue     | load an upvalue into the accumulator                                                                  |
| store_upvalue    | store the accumulator in an upvalue                                                                   |
| load_module_var  | load a module variable into the accumulator                                                           |
| store_module_var | store the accumulator into a module variable                                                          |
| load_global      | load a global into the accumulator                                                                    |
| store_global     | store the accumulator into a global                                                                   |
| load_field       | load a field into the accumulator, panics if the field does not exist                                 |
| load_field_opt   | load a field into the accumulator, yields `none` if the field does not exist                          |
| store_field      | store the accumulator into a field                                                                    |
| load_index       | load an index into the accumulator, panics if the index does not exist                                |
| load_index_opt   | load an index into the accumulator, yields `none` if the index does not exist                         |
| store_index      | store the accumulator into an index                                                                   |
| load_self        | load `self` into the accumulator                                                                      |
| load_super       | load the current super-class into the accumulator                                                     |
| load_none        | load `none` into the accumulator                                                                      |
| load_true        | load boolean `true` into the accumulator                                                              |
| load_false       | load boolean `false` into the accumulator                                                             |
| load_smi         | load an integer value into the accumulator                                                            |
| make_fn          | instantiate a function using a function descriptor                                                    |
| upvalue_reg      | capture a register into an upvalue in the function stored in the accumulator                          |
| upvalue_slot     | capture an upvalue from the parent function into an upvalue in the function stored in the accumulator |
| make_class       | instantiate a class using a class descriptor                                                          |
| jump             | jump forward by some byte offset                                                                      |
| loop             | jump backward by some byte offset                                                                     |
| jump_if_false    | jump forward by some byte offset if the accumulator holds a falsey value                              |
| add              | add a value stored in a register to the accumulator                                                   |
| sub              | subtract a value stored in a register from the accumulator                                            |
| mul              | multiply the accumulator by a value stored in a register                                              |
| div              | divide the accumulator by a value stored in a register                                                |
| rem              | divide the accumulator by a value stored in a register, and store the remainder in the accumulator    |
| pow              | raise the accumulator to the power of a value stored in a register                                    |
| inv              | invert the accumulator                                                                                |
| not              | negate the accumulator                                                                                |
| cmp_eq           | test if the accumulator is equal to a value stored in a register                                      |
| cmp_ne           | test if the accumulator is not equal to a value stored in a register                                  |
| cmp_gt           | test if the accumulator is greater than a value stored in a register                                  |
| cmp_ge           | test if the accumulator is greater than or equal to a value stored in a register                      |
| cmp_lt           | test if the accumulator is less than a value stored in a register                                     |
| cmp_le           | test if the accumulator is less than or equal to a value stored in a register                         |
| is               | test if the accumulator is an instance of a class                                                     |
| in               | test if the accumulator is contained in a value stored in a register                                  |
| print            | print the accumulator                                                                                 |
| print_list       | print values from `start` to `start+count`                                                            |
| call             | call a function                                                                                       |
| return           | return from a function call                                                                           |
| yield            | stop the dispatch loop, allowing it to be resumed later                                               |


## Function calls

TODO
- top of previous stack = call args (no copy)
  - implies that frames have to be able to overlap
  - what about coroutines?
    - is it enough to copy args in this case?
- class constructor = regular function call
  - has to be frozen after
- module root = regular function call
  - has to be set to initialized after

## Resources

- https://v8.dev/docs/ignition
- http://www.lua.org/source/5.1/lopcodes.h.html
