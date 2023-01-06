use std::collections::HashMap;
use std::hash::Hash;

use crate::opcode::{Chunk, Opcode};
use crate::u24::u24;

pub struct Builder<Value> {
  bytecode: Vec<Opcode>,
  const_pool: Vec<Value>,
  const_index_map: HashMap<Value, u24>,

  label_id: u24,
  reserved_labels: Vec<u24>,
}

impl<Value: Hash + Eq> Builder<Value> {
  pub fn new() -> Self {
    Self {
      bytecode: Vec::new(),
      const_pool: Vec::new(),
      const_index_map: HashMap::new(),

      label_id: u24::default(),
      reserved_labels: Vec::new(),
    }
  }

  /// Reserve a jump label.
  ///
  /// Because we don't know what the offset of a jump will be when the jump
  /// opcode is first inserted into the bytecode, we store a temporary value
  /// (the label) in place of its `offset`. When the bytecode is finalized,
  /// all labels are replaced with their real offset values.
  pub fn label(&mut self) -> u24 {
    let temp = self.label_id;
    self.reserved_labels.push(temp);
    self.label_id += 1;
    temp
  }

  /// Reserve N jump labels.
  ///
  /// See [`label`][`crate::builder::Builder::label`] for more information.
  pub fn labels<const N: usize>(&mut self) -> [u24; N] {
    let mut out = [u24::default(); N];
    for label in out.iter_mut() {
      *label = self.label();
    }
    out
  }

  /// Push an opcode into the bytecode array.
  pub fn opcode(&mut self, op: Opcode) {
    self.bytecode.push(op);
  }

  /// Inserts an entry into the constant pool, and returns the index.
  ///
  /// If `value` is already in the constant pool, this just returns its index.
  pub fn constant(&mut self, value: Value) -> Option<u24> {
    if let Some(index) = self.const_index_map.get(&value).cloned() {
      return Some(index);
    }

    let index = u24::try_from(self.const_pool.len() as u32).ok()?;
    self.const_pool.push(value);
    Some(index)
  }

  pub fn build(self) -> Chunk<Value> {
    todo!()
  }
}

impl<Value: Hash + Eq> Default for Builder<Value> {
  fn default() -> Self {
    Self::new()
  }
}
