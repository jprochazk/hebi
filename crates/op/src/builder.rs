use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use beef::lean::Cow;

use crate::chunk::{BytecodeArray, Chunk};
use crate::opcode::ty::Width;
use crate::opcode::*;

pub struct BytecodeBuilder<Value: Hash + Eq> {
  function_name: Cow<'static, str>,

  bytecode: BytecodeArray,
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
struct Label {
  id: u32,
  name: Cow<'static, str>,
  offset: Option<u32>,
}

impl<Value: Hash + Eq> BytecodeBuilder<Value> {
  pub fn new(function_name: impl Into<Cow<'static, str>>) -> Self {
    Self {
      function_name: function_name.into(),

      bytecode: BytecodeArray::new(),
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
  pub fn label(&mut self, name: Cow<'static, str>) -> u32 {
    let temp = self.label_id;
    self.label_map.insert(
      temp,
      Label {
        id: temp,
        name,
        offset: None,
      },
    );
    self.label_id += 1;
    temp
  }

  /// Reserve N jump labels.
  ///
  /// See [`label`][`crate::BytecodeBuilder::label`] for more information.
  pub fn labels<const N: usize, T: Into<Cow<'static, str>> + Clone>(
    &mut self,
    names: [T; N],
  ) -> [u32; N] {
    let mut out = [u32::default(); N];
    for (label, name) in out.iter_mut().zip(names.iter()) {
      *label = self.label(name.clone().into());
    }
    out
  }

  pub fn finish_label(&mut self, label: u32) {
    let jump_index = u32::try_from(self.bytecode.len())
      .map_err(|_| ())
      .expect("bytecode.len() exceeded u32::MAX"); // should be checked elsewhere
    let Some(entry) = self.label_map.get_mut(&label) else {
      panic!("invalid label ID: {label}");
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

  pub fn op<Op: Encode>(&mut self, _: Op, operands: Op::Operands) -> &mut Self {
    Op::encode(&mut self.bytecode, operands);
    self
  }

  /// Finalize the bytecode, and emit a [`Chunk`][`crate::Chunk`].
  ///
  /// Bytecode is finalized by:
  /// - Ensuring every reserved label has been finalized,
  /// - Patching control flow instructions (such as jumps) by replacing label
  ///   IDs with final offset values
  ///
  /// ### Panics
  ///
  /// If any reserved label has not been finalized. This is a programmer error,
  /// because a user should not be able to cause the compiler to emit invalid
  /// bytecode.
  pub fn build(mut self) -> Chunk<Value> {
    // ensure bytecode is terminated by `op_suspend`,
    // so that the dispatch loop stops
    if self.bytecode[self.bytecode.len() - 1] != ops::Suspend {
      self.op(Suspend, ());
    };

    patch_jumps(
      self.function_name.as_ref(),
      &mut self.bytecode,
      &self.label_map,
    );

    Chunk {
      name: self.function_name,
      bytecode: self.bytecode,
      const_pool: self.const_pool,
    }
  }
}

fn patch_jump<T: Opcode + Encode<Operands = u32>>(
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

fn patch_jumps(function_name: &str, bytecode: &mut BytecodeArray, label_map: &HashMap<u32, Label>) {
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

  let unused_labels = label_map
    .iter()
    .filter(|(_, v)| !used_labels.contains(v))
    .map(|(_, v)| v.clone())
    .collect::<Vec<_>>();
  if !unused_labels.is_empty() {
    for label in unused_labels.iter() {
      eprintln!("unused label: {label:?}");
    }
    panic!("bytecode in functon {function_name} had some unused labels (see above)");
  }
}

/* impl<Value: Hash + Eq> BytecodeBuilder<Value> {
  pub fn op_nop(&mut self) -> &mut Self {
    op::Nop::encode(&mut self.bytecode, ());
    self
  }
  pub fn op_suspend(&mut self) -> &mut Self {
    op::Suspend::encode(&mut self.bytecode, ());
    self
  }
  pub fn op_load_const(&mut self, slot: u32) -> &mut Self {
    op::LoadConst::encode(&mut self.bytecode, slot);
    self
  }
  pub fn op_load_reg(&mut self, reg: u32) -> &mut Self {
    op::LoadReg::encode(&mut self.bytecode, reg);
    self
  }
  pub fn op_store_reg(&mut self, reg: u32) -> &mut Self {
    op::StoreReg::encode(&mut self.bytecode, reg);
    self
  }
  pub fn op_jump(&mut self, offset: u32) -> &mut Self {
    op::Jump::encode(&mut self.bytecode, offset);
    self
  }
  pub fn op_jump_if_false(&mut self, offset: u32) -> &mut Self {
    op::JumpIfFalse::encode(&mut self.bytecode, offset);
    self
  }
  pub fn op_sub(&mut self, lhs: u32) -> &mut Self {
    op::Sub::encode(&mut self.bytecode, lhs);
    self
  }
  pub fn op_print(&mut self, reg: u32) -> &mut Self {
    op::Print::encode(&mut self.bytecode, reg);
    self
  }
  pub fn op_push_small_int(&mut self, value: i32) -> &mut Self {
    op::PushSmallInt::encode(&mut self.bytecode, value);
    self
  }
  pub fn op_create_empty_list(&mut self) -> &mut Self {
    op::CreateEmptyList::encode(&mut self.bytecode, ());
    self
  }
  pub fn op_list_push(&mut self, list: u32) -> &mut Self {
    op::ListPush::encode(&mut self.bytecode, list);
    self
  }
  pub fn op_ret(&mut self) -> &mut Self {
    op::Ret::encode(&mut self.bytecode, ());
    self
  }
} */
