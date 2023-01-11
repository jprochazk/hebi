- codegen
  - `Codegen` struct
  - nested `State` structs
  - use callback methods for dealing with `begin_scope`/`end_scope` and similar
- VM
  - reference counting (no proper GC or cycle detection for now)
  - Value type using pointer tagging or nan-boxing?

change disassembly to include bytes (but not pc)
add a way to disable variable width encoding and specify type manually
e.g. push_small_int <value:i8> 
