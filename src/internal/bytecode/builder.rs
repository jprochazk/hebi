use std::cell::{Cell, RefCell};
use std::hash::{Hash, Hasher};

use indexmap::IndexMap;

use super::opcode::symbolic::*;
use super::opcode::{self as op, Instruction, Opcode};
use super::operands::{Operand, Width};
use crate::internal::object::{Any, ClassDescriptor, FunctionDescriptor, Ptr, Str};
use crate::internal::value::constant::{Constant, NonNaNFloat};
use crate::span::Span;

#[derive(Default)]
pub struct BytecodeBuilder {
  bytecode: Vec<u8>,
  constant_pool_builder: ConstantPoolBuilder,
  unbound_jumps: usize,

  // TODO: encode spans into a flat buffer
  // use delta-encoding to make the bytes smaller
  // NOTE: it's fine to store the data in a flat buffer, because
  // we iterate over the bytecode array and each time we step
  // through bytecode,  we can also step the span buffer iterator.
  // for random access, positions in the buffer may be saved and
  // restored at a later point.
  spans: Vec<Span>,
}

pub struct BasicLabel {
  name: &'static str,
  referrer_offset: Cell<Option<usize>>,
}

pub struct MultiLabel {
  name: &'static str,
  labels: RefCell<Vec<BasicLabel>>,
}

pub trait Label: private::Sealed {
  fn name(&self) -> &'static str;
  fn is_used(&self) -> bool;
  fn set_referrer(&self, offset: usize);
  fn bind(self, builder: &mut BytecodeBuilder);
}

impl private::Sealed for BasicLabel {}
impl Label for BasicLabel {
  fn name(&self) -> &'static str {
    self.name
  }
  fn is_used(&self) -> bool {
    self.referrer_offset.get().is_some()
  }
  fn set_referrer(&self, offset: usize) {
    self.referrer_offset.set(Some(offset))
  }
  fn bind(self, builder: &mut BytecodeBuilder) {
    let Some(referrer_offset) = self.referrer_offset.get() else {
      panic!("label {} bound without a referrer", self.name);
    };
    let current_offset = builder.bytecode.len();
    assert!(
      current_offset > referrer_offset,
      "label {} used for backward jump",
      self.name
    );

    builder.patch_jump(
      referrer_offset,
      op::Offset((current_offset - referrer_offset) as u32),
    );
    builder.unbound_jumps -= 1;
  }
}

impl private::Sealed for MultiLabel {}
impl Label for MultiLabel {
  fn name(&self) -> &'static str {
    self.name
  }
  fn is_used(&self) -> bool {
    false
  }
  fn set_referrer(&self, offset: usize) {
    self.labels.borrow_mut().push(BasicLabel {
      name: self.name,
      referrer_offset: Cell::new(Some(offset)),
    })
  }
  fn bind(self, builder: &mut BytecodeBuilder) {
    for label in self.labels.take() {
      label.bind(builder)
    }
  }
}

#[derive(Clone)]
pub struct LoopHeader {
  offset: Cell<Option<usize>>,
}

impl BytecodeBuilder {
  pub fn new() -> Self {
    Self {
      bytecode: Vec::new(),
      constant_pool_builder: ConstantPoolBuilder::new(),
      unbound_jumps: 0,

      spans: Vec::new(),
    }
  }

  fn write(&mut self, instruction: impl Instruction, span: Span) {
    instruction.encode(&mut self.bytecode);
    self.spans.push(span);
  }

  /// Emit an instruction.
  pub fn emit(&mut self, instruction: impl Instruction, span: impl Into<Span>) {
    assert!(
      !instruction.is_jump(),
      "use `emit_jump` to emit jump instructions"
    );
    self.write(instruction, span.into());
  }

  /// Create an empty label.
  ///
  /// Used with `emit_jump`
  pub fn label(&self, name: &'static str) -> BasicLabel {
    BasicLabel {
      name,
      referrer_offset: Cell::new(None),
    }
  }

  pub fn multi_label(&self, name: &'static str) -> MultiLabel {
    MultiLabel {
      name,
      labels: RefCell::new(Vec::new()),
    }
  }

  pub fn bind_label(&mut self, label: impl Label) {
    label.bind(self)
  }

  /// Emit a jump instruction. The instruction will be emitted with a
  /// placeholder offset, and patched later when the `label` is bound.
  pub fn emit_jump(&mut self, label: &impl Label, span: impl Into<Span>) {
    assert!(
      !label.is_used(),
      "more than one instruction refers to label {}",
      label.name(),
    );

    // see [docs/emit.md#jump-instruction-encoding] for a description of how this
    // works.
    self.unbound_jumps += 1;
    label.set_referrer(self.bytecode.len());
    let offset = self.constant_pool_builder().reserve();
    self.write(
      Jump {
        offset: op::Offset(offset.0),
      },
      span.into(),
    )
  }

  pub fn emit_jump_if_false(&mut self, label: &impl Label, span: impl Into<Span>) {
    assert!(
      !label.is_used(),
      "more than one instruction refers to label {}",
      label.name(),
    );

    // see [docs/emit.md#jump-instruction-encoding] for a description of how this
    // works.
    self.unbound_jumps += 1;
    label.set_referrer(self.bytecode.len());
    let offset = self.constant_pool_builder().reserve();
    self.write(
      JumpIfFalse {
        offset: op::Offset(offset.0),
      },
      span.into(),
    )
  }

  /// Marks the current offset as a loop header and returns it for use as a
  /// target in `emit_jump_loop`.
  pub fn loop_header(&self) -> LoopHeader {
    LoopHeader {
      offset: Cell::new(None),
    }
  }

  pub fn bind_loop_header(&self, loop_header: &LoopHeader) {
    loop_header.offset.set(Some(self.bytecode.len()))
  }

  pub fn emit_jump_loop(&mut self, loop_header: &LoopHeader, span: impl Into<Span>) {
    assert!(loop_header.offset.get().is_some(), "unbound loop header");

    let relative_offset = (self.bytecode.len() - loop_header.offset.get().unwrap()) as u32;
    self.write(
      JumpLoop {
        offset: op::Offset(relative_offset),
      },
      span.into(),
    )
  }

  pub fn constant_pool_builder(&mut self) -> &mut ConstantPoolBuilder {
    &mut self.constant_pool_builder
  }

  pub fn finish(self) -> (Vec<u8>, Vec<Constant>) {
    (self.bytecode, self.constant_pool_builder.constants)
  }

  fn patch_jump(&mut self, referrer_offset: usize, relative_offset: op::Offset) {
    let encoded_width: Width;
    let op: Opcode;
    match Opcode::new(self.bytecode[referrer_offset]) {
      Opcode::Wide16 => {
        encoded_width = Width::Wide16;
        op = Opcode::new(self.bytecode[referrer_offset + 1]);
      }
      Opcode::Wide32 => {
        encoded_width = Width::Wide32;
        op = Opcode::new(self.bytecode[referrer_offset + 1]);
      }
      v @ (Opcode::Jump | Opcode::JumpIfFalse) => {
        encoded_width = Width::Normal;
        op = v;
      }
      v => panic!("attempted to patch instruction {v:?} as a forward jump"),
    };

    let has_prefix = !encoded_width.is_normal();
    let opcode_offset = referrer_offset + has_prefix as usize;
    let operand_offset = referrer_offset + 1 + has_prefix as usize;

    // if the actual offset does not fit in the encoded width, put it in the
    // reserved constant instead
    if relative_offset.width() > encoded_width {
      let constant_index =
        op::Constant(u32::decode(&self.bytecode[operand_offset..], encoded_width));
      self
        .constant_pool_builder()
        .commit(relative_offset, constant_index);
      // change the opcode to the Const variant
      let new_op = match op {
        Opcode::Jump => Opcode::JumpConst as u8,
        Opcode::JumpIfFalse => Opcode::JumpIfFalseConst as u8,
        _ => unreachable!(),
      };
      self.bytecode[opcode_offset] = new_op;
    } else {
      // encode the offset directly
      relative_offset.encode_into(&mut self.bytecode[operand_offset..], encoded_width);
    }
  }
}

struct PtrHash(Ptr<Any>);

impl Hash for PtrHash {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.0.ptr_hash(state)
  }
}

impl PartialEq for PtrHash {
  fn eq(&self, other: &Self) -> bool {
    self.0.ptr_eq(&other.0)
  }
}

impl Eq for PtrHash {}

#[derive(Default)]
pub struct ConstantPoolBuilder {
  // TODO: compact constants on finalization
  // Output a map of <old index -> new index> for patching.
  //
  // Allocate a new vec, start moving constants into it.
  // For each constant moved, leave behind an entry (abuse `Constant::Offset` for this) which holds
  // the new index. Ignore any holes.
  // Traverse the bytecode and patch the constant indices using the old constants vec
  // which now only holds `Offset` entries.
  // This traversal should be combined with register patching into a single fixup pass.
  //
  // Combine it with a slice-based reservation system which wouldn't actually allocate an entry
  // until it's committed in `patch_jump`. If the entry is unused, it is unreserved and may be
  // used by a different jump instruction. This way the first 256 entries aren't taken up
  // too quickly.
  //
  // It's probably worth it because most jumps will not use their reserved entries.
  constants: Vec<Constant>,
  ptr_map: IndexMap<PtrHash, usize>,
  float_map: IndexMap<NonNaNFloat, usize>,
}

impl ConstantPoolBuilder {
  pub fn new() -> Self {
    Self {
      constants: Vec::new(),
      ptr_map: IndexMap::new(),
      float_map: IndexMap::new(),
    }
  }

  pub fn insert(&mut self, value: impl InsertConstant) -> op::Constant {
    value.insert(self)
  }

  pub fn reserve(&mut self) -> op::Constant {
    let index = self.constants.len();
    self.constants.push(Constant::Reserved);
    op::Constant(index as u32)
  }

  pub fn commit(&mut self, value: impl InsertConstant, index: op::Constant) {
    value.insert_at(self, index);
  }
}

pub trait InsertConstant: private::Sealed {
  fn insert(self, builder: &mut ConstantPoolBuilder) -> op::Constant;
  fn insert_at(self, builder: &mut ConstantPoolBuilder, index: op::Constant);
}

mod private {
  pub trait Sealed {}
}

macro_rules! insert_constant_object {
  ($object_type:ty, $constant_variant:ident) => {
    impl private::Sealed for Ptr<$object_type> {}
    impl InsertConstant for Ptr<$object_type> {
      fn insert(self, builder: &mut ConstantPoolBuilder) -> op::Constant {
        let key = PtrHash(self.clone().into_any());
        if let Some(index) = builder.ptr_map.get(&key).copied() {
          op::Constant(index as u32)
        } else {
          let index = builder.constants.len();
          builder.constants.push(Constant::$constant_variant(self));
          builder.ptr_map.insert(key, index);
          op::Constant(index as u32)
        }
      }
      fn insert_at(self, builder: &mut ConstantPoolBuilder, constant: op::Constant) {
        builder.constants[constant.index()] = Constant::$constant_variant(self);
      }
    }
  };
}

insert_constant_object!(Str, String);
insert_constant_object!(FunctionDescriptor, Function);
insert_constant_object!(ClassDescriptor, Class);

impl private::Sealed for NonNaNFloat {}
impl InsertConstant for NonNaNFloat {
  fn insert(self, builder: &mut ConstantPoolBuilder) -> op::Constant {
    let index = if let Some(index) = builder.float_map.get(&self).copied() {
      index
    } else {
      let index = builder.constants.len();
      builder.constants.push(Constant::Float(self));
      builder.float_map.insert(self, index);
      index
    };
    op::Constant(index as u32)
  }
  fn insert_at(self, builder: &mut ConstantPoolBuilder, constant: op::Constant) {
    builder.constants[constant.index()] = Constant::Float(self);
  }
}

impl private::Sealed for op::Offset {}
impl InsertConstant for op::Offset {
  fn insert(self, builder: &mut ConstantPoolBuilder) -> op::Constant {
    let index = op::Constant(builder.constants.len() as u32);
    builder.constants.push(Constant::Offset(self));
    index
  }

  fn insert_at(self, builder: &mut ConstantPoolBuilder, constant: op::Constant) {
    builder.constants[constant.index()] = Constant::Offset(self);
  }
}

#[cfg(all(test, not(feature = "__miri")))]
mod tests;
