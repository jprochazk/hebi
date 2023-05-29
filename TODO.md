
- [x] remove `__miri` feature and put args in `xtask miri` instead (`--filter` for running only specific tests)
  - [x] should unlock `--all-features`
- [x] change ptr repr to use manual vtable + static object trait
- [x] remove Table `named_field`, it should be reserved for methods
- [x] move all field access/index access/etc. to delegate to the object trait
- [ ] vm fully async
  - [x] stackless import
  - [ ] stackless class init
    - [ ] make `init` a contextual keyword that doesn't require `fn` in classes
    - [ ] user cannot access initializer (to call it again)
          only way is to call the class type
    - [ ] remove `super.init()` and replace with calling the proxy (`super()`)
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
- [ ] ops on builtins <<<
- [x] methods on builtins
- [x] for iter loops (list)
- [ ] fix "invalid indentation" errors in parser which should be more specific
- [ ] all function types should have the same global type `Function`
- [ ] all class types (including native) should have the same global type `Type`
- [ ] class instances should walk the parent chain
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
