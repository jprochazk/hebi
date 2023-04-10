#![allow(unused_parens, clippy::unused_unit)]

#[macro_use]
mod macros;
pub mod ty;

use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::ptr::NonNull;

use beef::lean::Cow;
use paste::paste;
pub use ty::Width;
use ty::*;

instructions! {
  Instruction, ops,
  Handler, dispatch,
  Nop, Wide, ExtraWide,
  Ret, Suspend,
  disassemble, update_registers;

  // loads/stores
  /// Load constant into the accumulator.
  ///
  /// ### Operands
  /// - `slot` - index of value in the constant pool.
  LoadConst (slot: Const) = 3,
  /// Load register into the accumulator.
  ///
  /// ### Operands
  /// - `reg` - register index.
  LoadReg (reg: Reg),
  /// Store the accumulator in a register.
  ///
  /// ### Operands
  /// - `reg` - register index.
  StoreReg (reg: Reg),
  /// Load capture into the accumulator.
  ///
  /// ### Operands
  /// - `slot` - capture list index.
  LoadCapture (slot: uv),
  /// Store the accumulator in a capture.
  ///
  /// ### Operands
  /// - `slot` - capture list index.
  StoreCapture (slot: uv),
  /// Load a module variable into the accumulator.
  ///
  /// ### Operands
  /// - `slot` - module variable index.
  LoadModuleVar (slot: uv),
  /// Store the accumulator in a module variable.
  ///
  /// ### Operands
  /// - `slot` - module variable index.
  StoreModuleVar (slot: uv),
  /// Load a global into the accumulator.
  ///
  /// ### Operands
  /// - `name` - constant pool index of name.
  LoadGlobal (name: Const),
  /// Store the accumulator in a global.
  ///
  /// ### Operands
  /// - `name` - constant pool index of name.
  StoreGlobal (name: Const),
  /// Load an object's field into the accumulator.
  ///
  /// Panic if the object in the accumulator does not
  /// have a field with key `name`.
  ///
  /// ### Operands
  /// - `name` - constant pool index of name.
  LoadField (name: Const),
  /// Load an object's field into the accumulator.
  ///
  /// Load `none` into the accumulator if the object in
  /// the accumulator does not have a field with key `name`.
  ///
  /// ### Operands
  /// - `name` - constant pool index of name.
  LoadFieldOpt (name: Const),
  /// Store the accumulator in the field of an object.
  ///
  /// ### Operands
  /// - `name` - constant pool index of name.
  /// - `obj` - register index of target object.
  StoreField (name: Const, obj: Reg),
  /// Load a value from the object in the accumulator at the given index.
  ///
  /// Panic if the object in the accumulator does not
  /// have a field with key `key`.
  ///
  /// ### Operands
  /// - `key` - register index of key.
  LoadIndex (key: Reg),
  /// Load a value from the object in the accumulator at the given index.
  ///
  /// Load `none` into the accumulator if the object in
  /// the accumulator does not have a field with key `key`.
  ///
  /// ### Operands
  /// - `key` - register index of key.
  LoadIndexOpt (key: Reg),
  /// Store the accumulator in an object at the given index.
  ///
  /// ### Operands
  /// - `key` - register index of key.
  /// - `obj` - register index of target object.
  StoreIndex (key: Reg, obj: Reg),

  /// Load a module found at `path` into `dest`.
  ///
  /// ### Operands
  /// - `path` - constant pool index of the path.
  /// - `dest` - register index of the destination.
  Import (path: Const, dest: Reg),

  /// Load a symbol from the module found at `path` into `dest`.
  ///
  /// ### Operands
  /// - `path` - constant pool index of the path.
  /// - `name` - constant pool index of the name.
  /// - `dest` - register index of the destination.
  ImportNamed (path: Const, name: Const, dest: Reg),

  LoadSelf (),
  LoadSuper (),

  // values
  /// Push a `None` value into the accumulator.
  PushNone (),
  /// Push a boolean `true` value into the accumulator.
  PushTrue (),
  /// Push a boolean `false` value into the accumulator.
  PushFalse (),
  /// Push a 32-bit signed integer into the accumulator.
  ///
  /// ### Operands
  /// - `value` - integer value.
  PushSmallInt (value: sf32),
  /// Push an empty list into the accumulator.
  CreateEmptyList (),
  /// Push the value stored in the accumulator into a list.
  ///
  /// ### Operands
  /// - `list` - register index of list.
  PushToList (list: Reg),
  /// Push an empty dictionary into the accumulator.
  CreateEmptyDict (),
  /// Push the value stored in the accumulator into a dictionary.
  ///
  /// ### Operands
  /// - `key` - register index of key.
  /// - `dict` - register index of dict.
  InsertToDict (key: Reg, dict: Reg),
  /// Push the value stored in the accumulator into a dictionary.
  ///
  /// ### Operands
  /// - `name` - constant pool index of name.
  /// - `dict` - register index of dict.
  InsertToDictNamed (name: Const, dict: Reg),
  /// Create a function from `desc`.
  ///
  /// This initializes `captures` to all `none` values, and assigns the current module index.
  ///
  /// ### Operands
  /// - `desc` - constant pool index of descriptor.
  CreateFunction (desc: Const),
  /// Capture `reg` and store it in the captures of the closure stored in the accumulator.
  ///
  /// ### Operands
  /// - `reg` - register index of the captured register.
  /// - `slot` - slot in capture list of closure in the accumulator.
  CaptureReg (reg: Reg, slot: uv),
  /// Capture `slot` and store it in the captures of the closure stored in the accumulator.
  ///
  /// ### Operands
  /// - `parent_slot` - parent capture list index.
  /// - `self_slot` - slot in capture list of closure in the accumulator.
  CaptureSlot (parent_slot: uv, self_slot: uv),

  /// Create an empty class from `desc`.
  ///
  /// ### Operands
  /// - `desc` - constant pool index of descriptor.
  CreateClassEmpty (desc: Const),
  /// Create a class from `desc`, methods, and fields.
  ///
  /// Parent is store at `base+start`, if `desc.is_derived` is `true`.
  ///
  /// Methods are stored at `base+start..base+start+#methods`, offset by `1` if `desc.is_derived` is `true`.
  ///
  /// Fields are stored at `base+start+#methods..base+start+#methods+#fields`, offset by `1` if `desc.is_derived` is `true`.
  ///
  /// ### Operands
  /// - `desc` - constant pool index of descriptor.
  /// - `start` - register index of the first method.
  CreateClass (desc: Const, start: Reg),

  // jumps
  /// Jump forward by `offset`.
  Jump :jump (offset: uv),
  /// Jump backwards by `offset`.
  ///
  /// This instruction should not be emitted directly.
  JumpBack :jump (offset: uv),
  /// Jump forward by `offset` if value stored in the accumulator is truthy.
  JumpIfFalse :jump (offset: uv),

  // arithmetic (binary)
  /// Add `lhs` to value stored in the accumulator, and store the result in the accumulator.
  ///
  /// ### Operands
  /// - `lhs` - register index of the left-hand side expression.
  Add (lhs: Reg),
  /// Subtract value stored in the accumulator from `lhs`, and store the result in the accumulator.
  ///
  /// ### Operands
  /// - `lhs` - register index of the left-hand side expression.
  Sub (lhs: Reg),
  /// Multiply `lhs` by value stored in the accumulator, and store the result in the accumulator.
  ///
  /// ### Operands
  /// - `lhs` - register index of the left-hand side expression.
  Mul (lhs: Reg),
  /// Divide `lhs` by value stored in the accumulator, and store the result in the accumulator.
  ///
  /// ### Operands
  /// - `lhs` - register index of the left-hand side expression.
  Div (lhs: Reg),
  /// Divide `lhs` by value stored in the accumulator, and store the remainder of the division in the accumulator.
  ///
  /// ### Operands
  /// - `lhs` - register index of the left-hand side expression.
  Rem (lhs: Reg),
  /// Raise `lhs` to the power of N, where N is the value stored in the accumulator, and store the result in the accumulator.
  ///
  /// ### Operands
  /// - `lhs` - register index of the left-hand side expression.
  Pow (lhs: Reg),

  // unary
  // TODO: `value_of` override?
  /// Get the numerical value of the accumulator, and store it in the accumulator.
  UnaryPlus (),
  /// Get the numerical value of the accumulator, negate it, then store it in the accumulator.
  UnaryMinus (),
  /// Get the boolean value of the accumulator, negate it, then store it in the accumulator.
  UnaryNot (),

  // comparison
  /// Compare `lhs` to the accumulator.
  ///
  /// If `lhs` is equal to the accumulator, store `true` in the accumulator.
  /// Otherwise, store `false`.
  CmpEq (lhs: Reg),
  /// Compare `lhs` to the accumulator.
  ///
  /// If `lhs` is not equal to the accumulator, store `true` in the accumulator.
  /// Otherwise, store `false`.
  CmpNeq (lhs: Reg),
  /// Compare `lhs` to the accumulator.
  ///
  /// If `lhs` is greater than to the accumulator, store `true` in the accumulator.
  /// Otherwise, store `false`.
  CmpGt (lhs: Reg),
  /// Compare `lhs` to the accumulator.
  ///
  /// If `lhs` is greater than or equal to the accumulator, store `true` in the accumulator.
  /// Otherwise, store `false`.
  CmpGe (lhs: Reg),
  /// Compare `lhs` to the accumulator.
  ///
  /// If `lhs` is less than to the accumulator, store `true` in the accumulator.
  /// Otherwise, store `false`.
  CmpLt (lhs: Reg),
  /// Compare `lhs` to the accumulator.
  ///
  /// If `lhs` is less than or equal to the accumulator, store `true` in the accumulator.
  /// Otherwise, store `false`.
  CmpLe (lhs: Reg),

  // Type checks
  IsNone (),

  /// Print the value in the accumulator.
  Print (),
  /// Print a list of values.
  ///
  /// ### Operands
  /// - `list` - register index of value list.
  PrintList (list: Reg),

  /// Call `callee` using only positional arguments.
  ///
  /// The callee is stored in the accumulator.
  ///
  /// Call operation:
  /// 1. Assert that `callee` is callable, or panic.
  /// 2. Check the call arguments. [check]
  /// 3. Create a new call frame.
  /// 4. Initialize the function's params. [params]
  /// 5. Store the current call frame's IP, and dispatch on the new call frame.
  ///
  /// [check]: The following conditions must be true:
  /// - There are more than `callee.min_args` arguments.
  /// - There are less than `callee.max_args` arguments.
  ///
  /// [params]: Param initialization process
  /// - Set slot `[0]` (receiver) to `none`.
  /// - Copy the function to slot `[1]` (function).
  /// - If `num_args > max_args`, create a list at slot `[2]`, and initialize it with `args[max_args..]`.
  ///   Otherwise, set `[2]` to `none`.
  /// - Set slot `[3]` (kwargs) to `none`.
  /// - Copy arguments from `args[..num_args]` to `[4]..[4+N-1]`.
  /// - If `num_args < max_args`, initialize `params[4+num_args..4+max_args]` to `none`.
  ///
  /// Stack after initialization:
  /// ```text,ignore
  /// [0    ] <receiver>
  /// [1    ] <function>
  /// [2    ] argv
  /// [3    ] kwargs
  /// [4+0  ] params[0]
  /// [4+...] params[...]
  /// [4+N-1] params[N-1]
  /// ```
  ///
  /// ### Operands
  ///
  /// None.
  Call0 :call (),

  /// Call `callee` using only positional arguments.
  ///
  /// The callee is stored in the accumulator.
  ///
  /// The stack should be:
  /// ```text,ignore
  /// [...]
  /// 0   | args[0]
  /// *   | args[*]
  /// N-1 | args[N-1]
  /// ```
  /// `args[0]` is at `base+start`.
  ///
  /// Call operation:
  /// 1. Assert that `callee` is callable, or panic.
  /// 2. Check the call arguments. [check]
  /// 3. Create a new call frame.
  /// 4. Initialize the function's params. [params]
  /// 5. Store the current call frame's IP, and dispatch on the new call frame.
  ///
  /// [check]: The following conditions must be true:
  /// - There are more than `callee.min_args` arguments.
  /// - There are less than `callee.max_args` arguments.
  ///
  /// [params]: Param initialization process
  /// - Set slot `[0]` (receiver) to `none`.
  /// - Copy the function to slot `[1]` (function).
  /// - If `num_args > max_args`, create a list at slot `[2]`, and initialize it with `args[max_args..]`.
  ///   Otherwise, set `[2]` to `none`.
  /// - Set slot `[3]` (kwargs) to `none`.
  /// - Copy arguments from `args[..num_args]` to `[4]..[4+N-1]`.
  /// - If `num_args < max_args`, initialize `params[4+num_args..4+max_args]` to `none`.
  ///
  /// Stack after initialization:
  /// ```text,ignore
  /// [0    ] <receiver>
  /// [1    ] <function>
  /// [2    ] argv
  /// [3    ] kwargs
  /// [4+0  ] params[0]
  /// [4+...] params[...]
  /// [4+N-1] params[N-1]
  /// ```
  ///
  /// ### Operands
  /// - `start` - keyword dictionary register index.
  /// - `args` - number of positional arguments.
  Call :call (start: Reg, args: uv),

  /// Call `callee` with mixed positional and keyword arguments.
  ///
  /// The callee is stored in the accumulator.
  ///
  /// The stack should be:
  /// ```text,ignore
  /// [...]
  /// 0 | kw
  /// 1 | args[0]
  /// * | args[*]
  /// N | args[N-1]
  /// ```
  /// The `kw` is at `base+start`.
  ///
  /// Call operation:
  /// 1. Assert that `callee` is callable, or panic.
  /// 2. Check the call arguments. [check]
  /// 3. Create a new call frame.
  /// 4. Initialize the function's params. [params]
  /// 5. Store the current call frame's IP, and dispatch on the new call frame.
  ///
  /// [check]: The following conditions must be true:
  /// - `num_args >= callee.min_args && num_args <= callee.max_args`
  /// - All required keyword arguments appear in `kw`.
  ///
  /// [params]: Param initialization process
  /// - Set slot `[0]` (receiver) to `none`.
  /// - Copy the function to slot `[1]` (function).
  /// - If `num_args > max_args`, create a list at slot `[2]`, and initialize it with `args[max_args..]`.
  ///   Otherwise, set `[2]` to `none`.
  /// - Copy the `kw` dictionary to slot `[3]` (kwargs).
  /// - Copy arguments from `args[0..num_args]` to `params[4..4+num_args]`
  /// - If `num_args < max_args`, initialize `params[4+num_args..4+max_args]` to `none`.
  ///
  /// Stack after initialization:
  /// ```text,ignore
  /// [0    ] <receiver>
  /// [1    ] <function>
  /// [2    ] argv
  /// [3    ] kwargs
  /// [4+0  ] params[0]
  /// [4+...] params[...]
  /// [4+N-1] params[N-1]
  /// ```
  ///
  /// ### Operands
  /// - `start` - keyword dictionary register index.
  /// - `args` - number of positional arguments.
  CallKw :call (start: Reg, args: uv),

  /// Sets the accumulator to `true` if `call_frame.num_args <= n`.
  ///
  /// ### Operands
  /// - `n` - positional argument index
  IsPosParamNotSet (index: uv),
  /// Sets the accumulator to `true` if keyword argument `name` is not set.
  ///
  /// ### Operands
  /// - `name` - constant pool index of name.
  IsKwParamNotSet (name: Const),
  /// Load keyword argument `name` into `reg`.
  ///
  /// Keyword argument dictionary is stored in `call_frame.stack_base + 3`.
  ///
  /// This should remove `name` from the dictionary.
  ///
  /// ### Operands
  /// - `name` - constant pool index of name.
  /// - `param` - register index of function parameter.
  LoadKwParam (name: Const, param: Reg),
}

pub trait Opcode: private::Sealed {
  /// Returns the name of the operand for the purpose of `Display`.
  const NAME: &'static str;
}

pub trait Decode: private::Sealed {
  type Operands: Size;
  type Decoded: Sized;

  /// Decodes operands from `bytecode` at the given `offset`, scaling up
  /// variable-width operands by `width` as needed.
  fn decode(bytecode: &[u8], offset: usize, width: Width) -> Self::Decoded;
}

pub trait Encode: private::Sealed {
  /// Encode `self` in variable-width encoding.
  ///
  /// This emits a prefix byte, the opcode byte, and the operands in
  /// little-endian byte order.
  fn encode(&self, buf: &mut Vec<u8>, force_max_width: bool);
}

pub trait EncodeInto: Decode + private::Sealed {
  fn encode_into(buf: &mut [u8], operands: Self::Decoded);
}

pub enum ControlFlow {
  /// Dispatch an instruction after updating the program counter by adding
  /// `offset` from it.
  Jump(usize),
  /// Dispatch an instruction after updating the program counter by subtracting
  /// `offset` from it.
  Loop(usize),
  /// Stop the dispatch loop.
  Yield,
  /// Dispatch an instruction without updating the program counter.
  Nop,
}

pub struct Builder<Value: Hash + Eq + Clone> {
  function_name: Cow<'static, str>,

  instructions: Vec<Instruction>,
  /// Pool of constants referenced in the bytecode.
  const_pool: Vec<Value>,
  /// Map of constants to their indices in `const_pool`
  ///
  /// This is used to de-duplicate constants.
  const_index_map: HashMap<Value, u32>,

  /// Current unique label ID
  label_id: u32,
  /// Map of label IDs to jump indices.
  ///
  /// This is used to patch jump instruction offsets in `build`
  label_map: HashMap<u32, Label>,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Label {
  id: u32,
  name: Cow<'static, str>,
  offset: Option<u32>,
  allow_unused: bool,
}

#[derive(Clone, Copy)]
pub struct LabelId(u32);

impl LabelId {
  pub fn id(&self) -> u32 {
    self.0
  }
}

impl<Value: Hash + Eq + Clone> Builder<Value> {
  pub fn new(function_name: impl Into<Cow<'static, str>>) -> Self {
    Self {
      function_name: function_name.into(),

      instructions: Vec::new(),
      const_pool: Vec::new(),
      const_index_map: HashMap::new(),

      label_id: 0,
      label_map: HashMap::new(),
    }
  }

  /// Reserve a jump label.
  ///
  /// Each jump label must be finished using
  /// [`finish_label`][`crate::BytecodeBuilder::finish_label`], or explicitly
  /// allow it to be unused using
  /// [`allow_unused_label`][`crate::BytecodeBuilder::allow_unused_label`]
  /// before calling [`build`][`crate::BytecodeBuilder::build`]. Failing to do
  /// so will result in a panic.
  ///
  /// Because we don't know what the offset of a jump will be when the jump
  /// opcode is first inserted into the bytecode, we store a temporary value
  /// (the label) in place of its `offset`. When the bytecode is finalized,
  /// all labels are replaced with their real offset values.
  pub fn label(&mut self, name: impl Into<Cow<'static, str>>) -> LabelId {
    let id = self.label_id;
    self.label_map.insert(
      id,
      Label {
        id,
        name: name.into(),
        offset: None,
        allow_unused: false,
      },
    );
    self.label_id += 1;
    LabelId(id)
  }

  /// Reserve N jump labels.
  ///
  /// See [`label`][`crate::BytecodeBuilder::label`] for more information.
  pub fn labels<const N: usize, T: Into<Cow<'static, str>> + Clone>(
    &mut self,
    names: [T; N],
  ) -> [LabelId; N] {
    init_array_with(|i| self.label(names[i].clone().into()))
  }

  pub fn finish_label(&mut self, label: LabelId) {
    let jump_index = u32::try_from(self.instructions.len())
      .map_err(|_| ())
      .expect("bytecode.len() exceeded u32::MAX"); // should be checked elsewhere
    let Some(entry) = self.label_map.get_mut(&label.0) else {
      panic!("invalid label ID: {}", label.0);
    };
    entry.offset = Some(jump_index);
  }

  pub fn allow_unused_label(&mut self, label: LabelId) {
    let Some(entry) = self.label_map.get_mut(&label.0) else {
      panic!("invalid label ID: {}", label.0);
    };
    entry.allow_unused = true;
  }

  /// Inserts an entry into the constant pool, and returns the index.
  ///
  /// If `value` is already in the constant pool, this just returns its index.
  pub fn constant(&mut self, value: impl Into<Value>) -> u32 {
    let value = value.into();
    if let Some(index) = self.const_index_map.get(&value).cloned() {
      return index;
    }

    let index = self.const_pool.len() as u32;
    self.const_pool.push(value.clone());
    self.const_index_map.insert(value, index);
    index
  }

  pub fn op(&mut self, op: impl Into<Instruction>) {
    self.instructions.push(op.into());
  }

  pub fn patch(&mut self, f: impl FnOnce(&mut Vec<Instruction>)) {
    f(&mut self.instructions)
  }

  pub fn instructions(&self) -> &[Instruction] {
    &self.instructions
  }

  /// Finalize the bytecode, and emit a [`Chunk`][`crate::Chunk`].
  ///
  /// Bytecode is finalized by emitting the bytes required to run the specified
  /// instructions.
  ///
  /// ### Panics
  ///
  /// If any reserved label has not been finalized. This is a programmer error,
  /// because a user should not be able to cause the compiler to emit invalid
  /// bytecode.
  pub fn build(self) -> Chunk<Value> {
    let mut bytecode = Vec::new();
    let mut offsets = Vec::new();

    let name = self.function_name;
    let label_map = self.label_map;
    let mut instructions = self.instructions;
    let const_pool = self.const_pool;
    let mut used_labels = HashSet::new();

    // functions without a final `return` always return `none`
    /* if !matches!(instructions.last(), Some(Instruction::Ret(..))) {
      instructions.push(Instruction::PushNone(PushNone));
      instructions.push(Instruction::Ret(Ret));
    } */

    // ensure bytecode is terminated by `op_suspend`,
    // so that the dispatch loop stops
    if !matches!(instructions.last(), Some(Instruction::Suspend(..))) {
      instructions.push(Instruction::Suspend(Suspend));
    }

    // TODO: can we clear the `Nop`s produced by jump patching?
    // Should we switch to using basic blocks instead?

    // clear all `Nop`s
    // let instructions = instructions
    //   .into_iter()
    //   .filter(|op| !matches!(op, Instruction::Nop(..)))
    //   .collect::<Vec<_>>();

    // pass 1: variable-width encoding
    // jumps are encoded as max width
    for op in instructions.iter() {
      offsets.push(bytecode.len());
      match op {
        Instruction::Jump(..) => op.encode(&mut bytecode, true),
        Instruction::JumpBack(..) => op.encode(&mut bytecode, true),
        Instruction::JumpIfFalse(..) => op.encode(&mut bytecode, true),
        _ => op.encode(&mut bytecode, false),
      }
    }
    offsets.push(bytecode.len());

    // pass 2: patch jumps
    let mut ip = 0;
    while ip < bytecode.len() {
      match (bytecode.get(ip), bytecode.get(ip + 1)) {
        (Some(&ops::ExtraWide), Some(&ops::Jump)) => {
          let label_id = Jump::decode(&bytecode, ip + 2, Width::Quad);
          let offset = get_label_offset(label_id, &label_map, &offsets, &mut used_labels);
          match ip.cmp(&(offset as usize)) {
            std::cmp::Ordering::Greater | std::cmp::Ordering::Equal => {
              let offset = ip - offset as usize;
              patch_jump::<JumpBack>(&mut bytecode, ip, offset as u32);
            }
            std::cmp::Ordering::Less => {
              let offset = offset as usize - ip;
              patch_jump::<Jump>(&mut bytecode, ip, offset as u32);
            }
          }
        }
        (Some(&ops::ExtraWide), Some(&ops::JumpBack)) => {
          panic!("JumpBack should not be emitted directly ({ip})");
        }
        (Some(&ops::ExtraWide), Some(&ops::JumpIfFalse)) => {
          let label_id = JumpIfFalse::decode(&bytecode, ip + 2, Width::Quad);
          let offset = get_label_offset(label_id, &label_map, &offsets, &mut used_labels);
          if ip < offset as usize {
            let offset = offset as usize - ip;
            patch_jump::<JumpIfFalse>(&mut bytecode, ip, offset as u32);
          } else {
            panic!(
              "JumpIfFalse cannot go backwards (label {} ({label_id}) offset={offset})",
              &label_map[&label_id].name
            )
          }
        }
        _ => ip += decode_size(&bytecode[ip..]),
      }
    }

    let unused_labels = label_map
      .iter()
      .filter(|(_, v)| !v.allow_unused && !used_labels.contains(&v.id))
      .map(|(_, v)| v)
      .collect::<Vec<_>>();
    if !unused_labels.is_empty() {
      for label in unused_labels.iter() {
        eprintln!("unused label: {label:?}");
      }
      panic!("bytecode in function {name} had some unused labels (see above)");
    }

    Chunk {
      name,
      bytecode,
      const_pool,
    }
  }
}

fn get_label_offset(
  label_id: u32,
  label_map: &HashMap<u32, Label>,
  offsets: &[usize],
  used_labels: &mut HashSet<u32>,
) -> u32 {
  let label = label_map
    .get(&label_id)
    .unwrap_or_else(|| panic!("unknown label id {label_id}"));
  let index = label
    .offset
    .unwrap_or_else(|| panic!("unfinished label {} ({})", label.name, label.id));
  used_labels.insert(label.id);
  offsets[index as usize] as u32
}

fn patch_jump<T>(buf: &mut [u8], pc: usize, operands: T::Decoded)
where
  T: Decode + EncodeInto,
  <T as Decode>::Operands: Size,
{
  assert!(matches!(buf[pc], ops::ExtraWide));
  // clear it first, so that all the unused bytes become `nop` instructions
  buf[pc..pc + 2 + <T as Decode>::Operands::size(Width::Quad)].copy_from_slice(&[0u8; 6]);
  T::encode_into(&mut buf[pc..], operands)
}

fn init_array_with<T: Sized, const N: usize>(mut f: impl FnMut(usize) -> T) -> [T; N] {
  use std::mem::MaybeUninit;

  // SAFETY: It is safe to assume that `[MaybeUninit<T>; N]` is initialized,
  //         because its contents do not require initialization.
  #[allow(clippy::uninit_assumed_init)]
  let mut array: [MaybeUninit<T>; N] = unsafe { MaybeUninit::uninit().assume_init() };
  for (i, value) in array.iter_mut().enumerate() {
    MaybeUninit::write(value, f(i));
  }
  // SAFETY: The contents of `array` have been initialized, so it is safe to read from it.
  //         The code below is analogue to `std::mem::transmute::<_, [T; N]>(array)`,
  //         and we do this only because we cannot use `transmute` due to this issue:
  //           https://github.com/rust-lang/rust/issues/61956
  //         which is actually a more general lack of support for const generics in `transmute`.
  let out = unsafe { std::ptr::read(&mut array as *mut _ as *mut [T; N]) };
  std::mem::forget(array);
  out
}

pub struct Chunk<Value: Hash + Eq + Clone> {
  pub name: Cow<'static, str>,
  pub bytecode: Vec<u8>,
  /// Pool of constants referenced in the bytecode.
  pub const_pool: Vec<Value>,
}

#[allow(dead_code)]
impl<Value: std::fmt::Display + Hash + Eq + Clone> Chunk<Value> {
  pub fn disassemble(&self, print_bytes: bool) -> String {
    use std::fmt::Write;

    let mut f = String::new();

    {
      let f = &mut f;

      // name
      writeln!(f, "function \"{}\":", self.name).unwrap();
      writeln!(f, "  length: {}", self.bytecode.len()).unwrap();

      // constants
      if self.const_pool.is_empty() {
        writeln!(f, "  const_pool: <empty>").unwrap();
      } else {
        writeln!(f, "  const_pool (length={}):", self.const_pool.len()).unwrap();
        for (i, value) in self.const_pool.iter().enumerate() {
          writeln!(f, "    {i} = {value}").unwrap();
        }
      }

      // bytecode
      writeln!(f, "  code:").unwrap();
      let offset_align = self.bytecode.len().to_string().len();
      let mut pc = 0;
      while pc < self.bytecode.len() {
        let (size, instr) = disassemble(&self.bytecode[..], pc);

        let bytes = {
          let mut out = String::new();
          if print_bytes {
            for byte in self.bytecode[pc..pc + size].iter() {
              write!(&mut out, "{byte:02x} ").unwrap();
            }
            if size < 6 {
              for _ in 0..(6 - size) {
                write!(&mut out, "   ").unwrap();
              }
            }
          }
          out
        };

        writeln!(f, "    {pc:offset_align$} | {bytes}{instr}").unwrap();

        pc += size;
      }
    }

    f
  }
}

pub trait Disassemble {
  /// Disassemble a variable-width encoded `self` from `buf` at the specified
  /// `offset`.
  ///
  /// The `offset` should point to the prefix byte if there is one, and to the
  /// opcode if there isn't.
  fn disassemble(buf: &[u8], offset: usize, width: Width) -> Disassembly;
}

pub(super) enum DisassemblyOperandKind {
  Simple,
  Const,
  Reg,
}

pub(super) struct DisassemblyOperand {
  pub(super) name: &'static str,
  pub(super) value: Box<dyn std::fmt::Display>,
  pub(super) kind: DisassemblyOperandKind,
}

pub struct Disassembly {
  pub(super) name: &'static str,
  pub(super) width: Width,
  pub(super) operands: Vec<DisassemblyOperand>,
  pub(super) size: usize,
}

impl Disassembly {
  pub fn has_prefix(&self) -> bool {
    matches!(self.width, Width::Double | Width::Quad)
  }

  pub fn size(&self) -> usize {
    if self.has_prefix() {
      1 + self.size
    } else {
      self.size
    }
  }
}

impl ::std::fmt::Display for Disassembly {
  fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
    // print opcode + prefix
    write!(f, "{}{} ", self.width.as_str(), self.name)?;

    // print operands
    let mut iter = self.operands.iter().peekable();
    while let Some(DisassemblyOperand { name, value, kind }) = iter.next() {
      match kind {
        DisassemblyOperandKind::Simple => write!(f, "{name}={value}")?,
        DisassemblyOperandKind::Const => write!(f, "[{value}]")?,
        DisassemblyOperandKind::Reg => write!(f, "r{value}")?,
      }
      if iter.peek().is_some() {
        write!(f, ", ")?;
      }
    }
    Ok(())
  }
}

mod private {
  pub trait Sealed {}
}
