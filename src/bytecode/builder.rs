use std::cell::Cell;
use std::hash::{Hash, Hasher};

use indexmap::IndexMap;

use super::opcode::symbolic::*;
use super::opcode::{self as op, Instruction, Opcode};
use super::operands::{Operand, Width};
use crate::span::Span;
use crate::value::constant::{Constant, NonNaNFloat};
use crate::value::object;

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

pub struct Label {
  name: &'static str,
  referrer_offset: Cell<Option<usize>>,
}

pub struct LoopHeader {
  offset: usize,
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
  pub fn label(&self, name: &'static str) -> Label {
    Label {
      name,
      referrer_offset: Cell::new(None),
    }
  }

  pub fn bind_label(&mut self, label: Label) {
    let Some(referrer_offset) = label.referrer_offset.get() else {
      panic!("label {} bound without a referrer", label.name);
    };
    let current_offset = self.bytecode.len();
    assert!(
      current_offset > referrer_offset,
      "label {} used for backward jump",
      label.name
    );

    self.patch_jump(
      referrer_offset,
      op::Offset((current_offset - referrer_offset) as u32),
    );
    self.unbound_jumps -= 1;
  }

  /// Emit a jump instruction. The instruction will be emitted with a
  /// placeholder offset, and patched later when the `label` is bound.
  pub fn emit_jump(&mut self, label: &Label, span: impl Into<Span>) {
    assert!(
      label.referrer_offset.get().is_none(),
      "more than one instruction refers to label {} (referrers: {}, {})",
      label.name,
      label.referrer_offset.get().unwrap(),
      self.bytecode.len(),
    );

    // see `docs/instructions.md` for an description of how this works.
    self.unbound_jumps += 1;
    label.referrer_offset.set(Some(self.bytecode.len()));
    let offset = self.constant_pool_builder().reserve();
    self.write(
      Jump {
        offset: op::Offset(offset.0),
      },
      span.into(),
    )
  }

  pub fn emit_jump_if_false(&mut self, label: &Label, span: impl Into<Span>) {
    assert!(
      label.referrer_offset.get().is_none(),
      "more than one instruction refers to label {} (referrers: {}, {})",
      label.name,
      label.referrer_offset.get().unwrap(),
      self.bytecode.len(),
    );

    // see `docs/instructions.md` for an description of how this works.
    self.unbound_jumps += 1;
    label.referrer_offset.set(Some(self.bytecode.len()));
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
      offset: self.bytecode.len(),
    }
  }

  pub fn emit_jump_loop(&mut self, header: &LoopHeader, span: impl Into<Span>) {
    let relative_offset = (self.bytecode.len() - header.offset) as u32;
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

struct PtrHash(object::Any);

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
    impl private::Sealed for object::Ptr<$object_type> {}
    impl InsertConstant for object::Ptr<$object_type> {
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

insert_constant_object!(object::String, String);
insert_constant_object!(object::FunctionDescriptor, Function);
insert_constant_object!(object::ClassDescriptor, Class);

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

#[cfg(test)]
mod tests;
