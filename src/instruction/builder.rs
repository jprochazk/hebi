use std::cell::Cell;
use std::hash::{Hash, Hasher};

use indexmap::IndexMap;

use super::opcodes::{self as op, Instruction, JumpInstruction};
use crate::value::constant::{Constant, NonNaNFloat};
use crate::value::object;

#[derive(Default)]
pub struct BytecodeBuilder {
  bytecode: Vec<u8>,
  constant_pool_builder: ConstantPoolBuilder,
  unbound_jumps: usize,
}

#[derive(Default)]
pub struct Label {
  name: &'static str,
  referrer_offset: Cell<Option<usize>>,
}

impl Label {
  pub fn new(name: &'static str) -> Self {
    Self {
      name,
      referrer_offset: Cell::new(None),
    }
  }
}

impl BytecodeBuilder {
  pub fn new() -> Self {
    Self {
      bytecode: Vec::new(),
      constant_pool_builder: ConstantPoolBuilder::new(),
      unbound_jumps: 0,
    }
  }

  pub fn emit(&mut self, instruction: impl Instruction) {
    assert!(
      instruction.is_jump(),
      "use `emit_jump` to emit jump instructions"
    );
    instruction.encode(&mut self.bytecode);
  }

  pub fn emit_jump(&mut self, mut instruction: impl JumpInstruction, label: &Label) {
    assert!(
      label.referrer_offset.get().is_none(),
      "more than one instruction refers to label {} (referrers: {}, {})",
      label.name,
      label.referrer_offset.get().unwrap(),
      self.bytecode.len(),
    );

    self.unbound_jumps += 1;
    label.referrer_offset.set(Some(self.bytecode.len()));
    let offset = self.constant_pool_builder().reserve();
    instruction.update_offset(op::Offset(offset.0));
    instruction.encode(&mut self.bytecode)
  }

  // TODO: accept LoopHeader instead of Label here after finding out what it is :)
  pub fn emit_jump_back(&mut self, _: impl JumpInstruction, _: &Label) {
    todo!()
  }

  pub fn constant_pool_builder(&mut self) -> &mut ConstantPoolBuilder {
    &mut self.constant_pool_builder
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

  pub fn insert(&mut self, v: impl InsertConstant) -> op::Constant {
    v.insert(self)
  }

  pub fn reserve(&mut self) -> op::Constant {
    let index = self.constants.len();
    self.constants.push(Constant::Reserved);
    op::Constant(index as u32)
  }
}

pub trait InsertConstant {
  fn insert(self, builder: &mut ConstantPoolBuilder) -> op::Constant;
}

/*
pub enum Constant {
  Reserved,
  String(Ptr<String>),
  Function(Ptr<FunctionDescriptor>),
  Class(Ptr<ClassDescriptor>),
  Float(NonNaNFloat),
}
*/

macro_rules! insert_constant_object {
  ($object_type:ty, $constant_variant:ident) => {
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
    }
  };
}

insert_constant_object!(object::String, String);
insert_constant_object!(object::FunctionDescriptor, Function);
insert_constant_object!(object::ClassDescriptor, Class);

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
}
