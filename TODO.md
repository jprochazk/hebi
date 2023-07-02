

- [x] go through usage of scopes and ensure all are being exited properly
- [x] fix native stuff
- [ ] vm fully async
  - [x] stackless import
  - [ ] stackless class init
    - [x] make `init` a contextual keyword that doesn't require `fn` in classes
    - [x] user cannot access initializer (to call it again)
          only way is to call the class type
    - [ ] remove `super.init()` and replace with calling the proxy (`super()`)

- [x] remove `__miri` feature and put args in `xtask miri` instead (`--filter` for running only specific tests)
  - [x] should unlock `--all-features`
- [x] change ptr repr to use manual vtable + static object trait
- [x] remove Table `named_field`, it should be reserved for methods
- [x] move all field access/index access/etc. to delegate to the object trait
- [x] fix `from m import a, b, c` bug
- [x] unify globals/global
- [x] remove `Ref` from name of public value types (inner should be prefix by `Owned` or qualified path)
- [x] fix `scope.params` will panic if given the wrong number of args
- [x] comma between disassembly operands
- [x] rename `emit` to `codegen`
- [x] print to configurable writer
- [x] allow printing disassembly (add option to compile snippet and explicitly run it -> expose `disassemble` on it)
- [x] list indexing
- [x] list index oob should return None instead of error (improve error message)
- [x] to_index should check MIN_SAFE_INT
- [x] methods on builtins
- [x] for iter loops (list)
- [ ] ops on builtins <<<
- [ ] store `this` as a special register in `Thread`
      - method calls can set `this` and `self` access will fetch through `this` as opposed to slot 0
      - `this` can be pushed onto the stack if it needs to be saved, such as script->script function calls
      - native/builtin functions can also access `this` through their `scope` as opposed to expecting it in param 0
- [ ] fix "invalid indentation" errors in parser which should be more specific
- [ ] all function types should have the same global type `Function` for use in `is` checks
- [ ] all class types (including native) should have the same global type `Type` for use in `is` checks
- [ ] class instances should walk the parent chain in `is` checks
- [ ] derive(Data)
  - immutable
  - non-constructible
  - no methods
- [ ] repl
  - multi-line editor
- [ ] spaces only, make better error message for tabs

- [ ] debugger
  - egui
  - inspect state of VM
    - call frames
    - stack, accumulator
    - pretty-print values (using Debug)
  - step through bytecode (not necessarily source code)

- [ ] tuples
- [ ] generators
- [ ] f-strings
- [ ] `is`
- [ ] `in`
- [ ] exceptions (try/catch)
  - [ ] script-land inheritable error type
- [ ] inherit from native class
- [ ] report code locations (intern spans, track in hashmap of `offset->span_id`)
- [ ] timeout feature (abort signal, `Arc<AtomicBool>` or similar)
- [ ] semicolons (`;` for stmt, `;;` for block)
- [ ] codegen optimizations
  - [ ] dead code elimination (using basic blocks)
  - [ ] constant pool compaction
  - [ ] elide previous instruction for clobbered reads
  - [ ] peephole previous 2 instructions
    - [ ] load + store -> mov
    - [ ] load_field + call -> invoke
    - [ ] (load_smi 1) + add -> add1
  - [ ] specializations
    - [ ] load rN -> loadN, for N in 0 to some max bound
    - [ ] store rN -> storeN, for N in 0 to some max bound
- [ ] inspect dispatch loop codegen (should be a jump table)
- [ ] inline caching
  - per-function IC
  - reserve IC slot in codegen
- [ ] quickening


## repro
https://haste.zneix.eu/wucivywofu.swift



## main/call
```
thread.main(function: Ptr<Function>) async
  push_frame(function, &[])
  run().await
  pop_frame()
  take(self.acc)

thread.run() async
  loop
    frame = current()
    match dispatch(frame)
      Poll(future) -> self.acc = future.await
      Yield -> break
    

thread.call(function: Ptr<Any>, args) -> Result<Value> async
  match function.call(get_empty_scope(), args)
    Return(value) -> value
    Poll(future) -> future.await
    Dispatch ->
      run().await
      take(self.acc)

// TODO
thread.op_call(callee, count)


trait Object
  call(scope, this: Ptr<Self>, args: Args) -> Result<Call>


impl for Ptr<Function>
  call(scope, this: Ptr<Function>, args: Args) -> Result<Call>
    check_args?
    if !has_self
      scope.stack.push(this)
    scope.stack.extend_from_within(args.to_range())
    scope.stack.extend(frame_size - args.count - (has_self as usize))
    push_frame()

enum Call
  Return(value)
  Poll(future)
  Dispatch
```
  




### class type trait
can be derived or implemented manually
```
trait Type {
  fn build(&mut ClassBuilder) -> ClassDescriptor;
}

NativeModuleBuilder
  .class(str, impl Type)

```


### IntoValue changes
users may only store userdata which is Send, so intovalue does not have to require send, because the only way to create a value is
out of Send things.

public API accepts impl IntoValue




### error reporting

Map code offset -> span

also need to know which file we're in so we can report


### codegen comparison to V8

```
LdaZero                                       ;   a = 0
Star0                                         ;   

LdaSmi [1]                                    ;   b = 1
Star1                                         ;   

LdaZero                                       ;   i = 0
Star2                                         ;   

                                              ; loop:
Ldar a0                                       ;   i < n
TestLessThan r2, [0]                          ;   

JumpIfFalse [24] (0x120efd313ffa @ 36)        ;   jump? .end

Ldar r1                                       ;   temp = a + b
Add r0, [1]                                   ;   
Star3                                         ;   

Mov r1, r0                                    ;   a = b

Mov r3, r1                                    ;   b = temp

Ldar r2                                       ;   i += 1
AddSmi [1], [2]                               ;   
Star2                                         ;   

JumpLoop [25], [0], [3] (0x120efd313fdd @ 7)  ;   jump .loop
                                              ; end:

Ldar r0                                       ;   return a
Return                                        ;   



load_smi 0        ;   a = 0
store r2          ;   

load_smi 1        ;   b = 1
store r3          ;   

load_smi 0        ;   i = 0
store r4          ;   

                  ; loop:
load r1           ;   n0 = n        # why isn't it directly using `n`?
store r5          ;   
load r5           ;   
cmp_lt r4         ;   i < n

jump_if_false 32  ;   jump? .end

jump 10           ;   jump .body    # numerical `for` should put the latch at end
                                    # which would remove `jump.body` and `jump .loop` here
                                    # the ending `jump .latch` would turn to `jump .loop`

                  ; latch:
load_smi 1        ;   i += 1
add r4            ;   
store r4          ;   

jump_loop 14      ;   jump .loop

                  ; body:
load r2           ;   temp = a      # probably same problem as `n0` above
store r6          ;   

load r3           ;   temp += b
add r6            ;   
store r6          ;   

load r3           ;   a = b         # need a `mov` that's a peephole of `load, store`
store r2          ;   

load r6           ;   b = temp      # also mov
store r3          ;   

jump_loop 26      ;   jump .latch

load r2           ;   return a
return            ;   

load_none         ;   return none   # basic block DCE would eliminate this
return            ;   
```
