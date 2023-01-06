- begin work on codegen
  - `Emitter` struct
  - `State` enum or just a struct? It can get pretty messy
  - use callback methods for dealing with stuff like `begin_scope`/`end_scope`
- extended opcodes
  - each simple opcode may hold up to 24 bits
  - each extended opcode may hold an arbitrary number of bits, and may encode arbitrary data
  - needs a high-level interface for both emit and decode
    in order to ensure extended opcodes cannot be used incorrectly
  - how would this work with serialized bytecode? is it even possible?
