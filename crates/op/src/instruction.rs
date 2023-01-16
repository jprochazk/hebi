#![allow(unused_parens, clippy::unused_unit)]

#[macro_use]
mod macros;
pub mod ty;

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use beef::lean::Cow;
use paste::paste;
use ty::*;

// TODO: instruction_list instead of individual instructions
// - instruction enum for emit and patching, then deprecate encode_into
// - enum should be used to define `BYTE`

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

fn handle_jump<E>(
  value: Result<ControlFlow, E>,
  pc: &mut usize,
  size_of_operands: usize,
  result: &mut Result<(), E>,
) {
  let _jump = match value {
    Ok(jump) => jump,
    Err(e) => {
      *result = Err(e);
      ControlFlow::Next
    }
  };
  match _jump {
    ControlFlow::Next => *pc += 1 + size_of_operands,
    ControlFlow::Goto(offset) => *pc = offset as usize,
  }
}

instructions! {
  Instruction, ops,
  Handler, run,
  Nop, Wide, ExtraWide, Suspend,
  disassemble;

  LoadConst (slot:uv) = 3,
  LoadReg (reg:uv),
  StoreReg (reg:uv),
  Jump :jump (offset:uv),
  JumpBack :jump (offset:uv),
  JumpIfFalse :jump (offset:uv),
  Add (lhs:uv),
  Sub (lhs:uv),
  Mul (lhs:uv),
  Div (lhs:uv),
  Rem (lhs:uv),
  Pow (lhs:uv),
  CmpEq (lhs:uv),
  CmpNeq (lhs:uv),
  CmpGt (lhs:uv),
  CmpGe (lhs:uv),
  CmpLt (lhs:uv),
  CmpLe (lhs:uv),
  // Invert (),
  Print (),
  PrintList (list:uv),
  PushNone (),
  PushTrue (),
  PushFalse (),
  PushSmallInt (value:sf32),
  CreateEmptyList (),
  PushToList (list:uv),
  CreateEmptyDict (),
  InsertToDict (dict:uv),
  // Call (),
  Ret (),
  // Suspend (),
}

// TODO: more instructions
// TODO: see how V8 handles `??` and `a?.b`
// float, bigint??, string -> constants
// TODO: calling convention
// TODO: fast call? (speed up keyword args) - maybe IC is enough?
// TODO: What does V8 do with call ICs?

// Nop
// Wide
// ExtraWide
// LoadConst
// LoadReg
// StoreReg
// Jump
// JumpBack
// JumpIfFalse
// Add
// Sub
// Mul
// Div
// Rem
// Pow
// Eq
// Neq
// Gt
// Ge
// Lt
// Le
// Invert
// Print
// PrintList
// PushNone
// PushTrue
// PushFalse
// PushSmallInt
// CreateEmptyList
// ListPush
// CreateEmptyDict
// DictInsert
// Call
// Ret
// Suspend

pub enum ControlFlow {
  /// Jump to some `offset` in the bytecode.
  ///
  /// Note: This must land the dispatch loop on a valid opcode.
  Goto(u32),
  /// Go to the next instruction.
  ///
  /// This is equivalent to
  /// `ControlFlow::Goto(pc + 1 + size_of_operands(opcode))`.
  Next,
}

pub struct Builder<Value: Hash + Eq> {
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Label {
  pub id: u32,
  pub name: Cow<'static, str>,
  pub offset: Option<u32>,
}

#[derive(Clone, Copy)]
pub struct LabelId(u32);

impl LabelId {
  pub fn id(&self) -> u32 {
    self.0
  }
}

impl<Value: Hash + Eq> Builder<Value> {
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
  /// [`finish_label`][`crate::BytecodeBuilder::finish_label`] before calling
  /// [`build`][`crate::BytecodeBuilder::build`]. Failing to do so will result
  /// in a panic.
  ///
  /// Because we don't know what the offset of a jump will be when the jump
  /// opcode is first inserted into the bytecode, we store a temporary value
  /// (the label) in place of its `offset`. When the bytecode is finalized,
  /// all labels are replaced with their real offset values.
  pub fn label(&mut self, name: Cow<'static, str>) -> LabelId {
    let id = self.label_id;
    self.label_map.insert(
      id,
      Label {
        id,
        name,
        offset: None,
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

  /// Inserts an entry into the constant pool, and returns the index.
  ///
  /// If `value` is already in the constant pool, this just returns its index.
  pub fn constant(&mut self, value: Value) -> u32 {
    if let Some(index) = self.const_index_map.get(&value).cloned() {
      return index;
    }

    let index = self.const_pool.len() as u32;
    self.const_pool.push(value);
    index
  }

  pub fn op(&mut self, op: impl Into<Instruction>) -> &mut Self {
    self.instructions.push(op.into());
    self
  }

  pub fn patch(&mut self, f: impl FnOnce(&mut Vec<Instruction>)) {
    f(&mut self.instructions)
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
          patch_jump::<Jump>(&mut bytecode, ip, offset);
        }
        (Some(&ops::ExtraWide), Some(&ops::JumpBack)) => {
          let label_id = JumpBack::decode(&bytecode, ip + 2, Width::Quad);
          let offset = get_label_offset(label_id, &label_map, &offsets, &mut used_labels);
          patch_jump::<JumpBack>(&mut bytecode, ip, offset);
        }
        (Some(&ops::ExtraWide), Some(&ops::JumpIfFalse)) => {
          let label_id = JumpIfFalse::decode(&bytecode, ip + 2, Width::Quad);
          let offset = get_label_offset(label_id, &label_map, &offsets, &mut used_labels);
          patch_jump::<JumpIfFalse>(&mut bytecode, ip, offset);
        }
        _ => ip += decode_size(&bytecode[ip..]),
      }
    }

    let unused_labels = label_map
      .iter()
      .filter(|(_, v)| !used_labels.contains(&v.id))
      .map(|(_, v)| v.clone())
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
  let mut array: [_; N] = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
  for (i, value) in array.iter_mut().enumerate() {
    *value = std::mem::MaybeUninit::new(f(i));
  }
  let out = unsafe { std::ptr::read(&mut array as *mut _ as *mut [T; N]) };
  std::mem::forget(array);
  out
}

/* fn patch_jump<T: Opcode + Encode<Operands = u32>>(
  _: T,
  buf: &mut [u8],
  offset: usize,
  jump_offset: u32,
) {
  assert!(T::IS_JUMP && matches!(buf[offset], ops::ExtraWide));
  // clear it first, so that all the unused bytes become `nop` instructions
  buf[offset..offset + 2 + T::size_of_operands(Width::Quad)].copy_from_slice(&[0u8; 6]);
  T::encode_into(buf, offset, jump_offset)
}

// TODO: patching API instead of this
// (will be used for both jumps and load/store registers)

fn patch_jumps(function_name: &str, bytecode: &mut Vec<u8>, label_map: &HashMap<u32, Label>) {
  let mut used_labels = HashSet::new();
  for pc in 0..bytecode.len() {
    let op = bytecode[pc];
    if is_jump(op) {
      let prefix_pc = pc - 1;
      // all jump instructions are emitted with `xwide` prefix by default,
      // then narrowed based on the final offset value
      assert!(matches!(bytecode[prefix_pc], ops::ExtraWide));

      // read the label id stored as offset
      let label_id = match op {
        ops::Jump => Jump::decode(&bytecode[..], pc + 1, Width::Quad),
        ops::JumpIfFalse => JumpIfFalse::decode(&bytecode[..], pc + 1, Width::Quad),
        _ => unreachable!("op::is_jump(0x{op:02x}) is true, but label_id is not being decoded"),
      };

      // find the label and get its final offset
      let label = label_map
        .get(&label_id)
        .unwrap_or_else(|| panic!("unknown label ID {label_id}"));
      let jump_offset = label
        .offset
        .unwrap_or_else(|| panic!("unfinished label `{}` ({})", label.name, label.id));
      used_labels.insert(label.clone());

      // patch the instruction
      match op {
        ops::Jump => patch_jump(Jump, bytecode, prefix_pc, jump_offset),
        ops::JumpIfFalse => patch_jump(JumpIfFalse, bytecode, prefix_pc, jump_offset),
        _ => unreachable!(
          "op::is_jump(0x{op:02x}) is true, and `op` was a jump instruction, but now it isn't"
        ),
      }
    }
  }

} */

pub struct Chunk<Value: Hash + Eq> {
  pub name: Cow<'static, str>,
  pub bytecode: Vec<u8>,
  /// Pool of constants referenced in the bytecode.
  pub const_pool: Vec<Value>,
}

impl<Value: std::fmt::Display + Hash + Eq> Chunk<Value> {
  pub fn disassemble(&self) -> String {
    use std::fmt::Write;

    let mut f = String::new();

    {
      let f = &mut f;

      // name
      writeln!(f, "function <{}>:", self.name).unwrap();
      writeln!(f, "length = {}", self.bytecode.len()).unwrap();

      // constants
      if self.const_pool.is_empty() {
        writeln!(f, "const pool = <empty>").unwrap();
      } else {
        writeln!(f, "const pool = (length={}) {{", self.const_pool.len()).unwrap();
        for (i, value) in self.const_pool.iter().enumerate() {
          writeln!(f, "  {i} = {value}").unwrap();
        }
        writeln!(f, "}}").unwrap();
      }

      // bytecode
      writeln!(f, "bytecode:").unwrap();
      let offset_align = self.bytecode.len().to_string().len();
      let mut pc = 0;
      while pc < self.bytecode.len() {
        let instr = disassemble(&self.bytecode[..], pc);
        let size = instr.size();

        let bytes = {
          let mut out = String::new();
          // print bytes
          for byte in self.bytecode[pc..pc + size].iter() {
            write!(&mut out, "{byte:02x} ").unwrap();
          }
          if size < 6 {
            for _ in 0..(6 - size) {
              write!(&mut out, "   ").unwrap();
            }
          }
          out
        };

        writeln!(f, " {pc:offset_align$} | {bytes}{instr}").unwrap();

        pc += size;
      }
    }

    f
  }
}

// TODO:
// - accumulator usage
// - operand usage instead of name e.g. to print `r0` instead of `reg=0`, `[0]`
//   instead of `slot=0`, etc.

pub trait Disassemble {
  /// Disassemble a variable-width encoded `self` from `buf` at the specified
  /// `offset`.
  ///
  /// The `offset` should point to the prefix byte if there is one, and to the
  /// opcode if there isn't.
  fn disassemble(buf: &[u8], offset: usize, width: Width) -> Disassembly;
}

fn align() -> usize {
  Instruction::names()
    .iter()
    .map(|v| v.len())
    .max()
    .unwrap_or(0)
}

pub(super) struct DisassemblyOperand {
  pub(super) name: &'static str,
  pub(super) value: Box<dyn std::fmt::Display>,
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
    write!(f, "{}{}", self.width.as_str(), self.name)?;

    // print operands
    write!(
      f,
      "{:w$}",
      "",
      w = align() - self.width.as_str().len() - self.name.len()
    )?;
    for DisassemblyOperand { name, value } in self.operands.iter() {
      write!(f, " {name}={value}")?;
    }
    Ok(())
  }
}

mod private {
  pub trait Sealed {}
}
