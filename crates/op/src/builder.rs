use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use beef::lean::Cow;

use crate::opcode::{Chunk, Opcode};
use crate::u24::u24;

pub struct Builder<Value> {
  function_name: Cow<'static, str>,

  bytecode: Vec<Opcode>,
  /// Pool of constants referenced in the bytecode.
  const_pool: Vec<Value>,
  /// Map of constants to their indices in `const_pool`
  ///
  /// This is used to de-duplicate constants.
  const_index_map: HashMap<Value, u24>,

  /// Current unique label ID
  label_id: u24,
  /// Map of label IDs to jump indices.
  ///
  /// This is used to patch jump instruction offsets in `build`
  label_map: HashMap<u24, Label>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Label {
  id: u24,
  name: Cow<'static, str>,
  jump_index: Option<u24>,
}

impl<Value: Hash + Eq> Builder<Value> {
  pub fn new(function_name: impl Into<Cow<'static, str>>) -> Self {
    Self {
      function_name: function_name.into(),

      bytecode: Vec::new(),
      const_pool: Vec::new(),
      const_index_map: HashMap::new(),

      label_id: u24::default(),
      label_map: HashMap::new(),
    }
  }

  /// Reserve a jump label.
  ///
  /// Because we don't know what the offset of a jump will be when the jump
  /// opcode is first inserted into the bytecode, we store a temporary value
  /// (the label) in place of its `offset`. When the bytecode is finalized,
  /// all labels are replaced with their real offset values.
  pub fn label(&mut self, name: Cow<'static, str>) -> u24 {
    let temp = self.label_id;
    self.label_map.insert(
      temp,
      Label {
        id: temp,
        name,
        jump_index: None,
      },
    );
    self.label_id += 1;
    temp
  }

  /// Reserve N jump labels.
  ///
  /// See [`label`][`crate::builder::Builder::label`] for more information.
  pub fn labels<const N: usize, T: Into<Cow<'static, str>> + Clone>(
    &mut self,
    names: [T; N],
  ) -> [u24; N] {
    let mut out = [u24::default(); N];
    for (label, name) in out.iter_mut().zip(names.iter()) {
      *label = self.label(name.clone().into());
    }
    out
  }

  pub fn finish_label(&mut self, label: u24) {
    let jump_index = u32::try_from(self.bytecode.len())
      .map_err(|_| ())
      .and_then(u24::try_from)
      .expect("bytecode.len() exceeded u24::MAX"); // should be checked elsewhere
    let Some(entry) = self.label_map.get_mut(&label) else {
      panic!("invalid label ID: {label}");
    };
    entry.jump_index = Some(jump_index);
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

  pub fn build(mut self) -> Chunk<Value> {
    patch_jumps(
      self.function_name.as_ref(),
      &mut self.bytecode[..],
      &self.label_map,
    );

    Chunk {
      bytecode: self.bytecode,
      const_pool: self.const_pool,
    }
  }
}

fn patch_jumps(function_name: &str, bytecode: &mut [Opcode], label_map: &HashMap<u24, Label>) {
  let mut used_labels = HashSet::new();
  for opcode in bytecode {
    match opcode {
      Opcode::Jump { offset } | Opcode::JumpIfFalse { offset } => {
        let label = label_map
          .get(offset)
          .unwrap_or_else(|| panic!("unknown label ID {offset}"));
        let jump_index = label
          .jump_index
          .unwrap_or_else(|| panic!("unfinished label `{}` ({})", label.name, label.id));
        used_labels.insert(label.clone());

        *offset = jump_index;
      }
      _ => {}
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
    panic!("bytecode in functon {function_name} had some unused labels");
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_patch_jumps() {
    let mut builder = Builder::<()>::new("test");

    let [start, exit] = builder.labels(["start", "exit"]);

    // exit:
    // end:
    // start:
    builder.finish_label(start);
    //     <condition>
    builder.bytecode.push(Opcode::PushBool { v: true });
    //     jump_if_false @exit
    builder.bytecode.push(Opcode::JumpIfFalse { offset: exit });
    //     pop condition result
    builder.bytecode.push(Opcode::Pop);
    //     <body>
    builder.bytecode.push(Opcode::Noop);
    //     jump @start
    builder.bytecode.push(Opcode::Jump { offset: start });
    // exit:
    builder.finish_label(exit);
    //     pop condition result
    builder.bytecode.push(Opcode::Pop);
    // end:
    //     ...

    let chunk = builder.build();
    assert_eq!(
      chunk.bytecode,
      vec![
        Opcode::PushBool { v: true },
        Opcode::JumpIfFalse {
          offset: 5_u8.into()
        },
        Opcode::Pop,
        Opcode::Noop,
        Opcode::Jump {
          offset: 0_u8.into()
        },
        Opcode::Pop
      ]
    );
  }
}
