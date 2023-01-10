#![allow(
  clippy::just_underscores_and_digits,
  non_upper_case_globals,
  clippy::needless_range_loop
)]

//#[macro_use]
//mod macros;

use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use beef::lean::Cow;
use op_macros::define_bytecode;

// Do not re-order instructions, unless you absolutely have to!
// If you want to add new instructions, add them to the end of the list.
define_bytecode! {
  /// Load a constant into the accumulator.
  load_const <slot>,
  /// Load a register into the accumulator.
  load_reg <reg>,
  /// Load the accumulator into a register.
  store_reg <reg>,
  /// Jump to the specified offset.
  jump :jump <offset>,
  /// Jump to the specified offset if the value in the accumulator is falsey.
  jump_if_false :jump <offset>,
  /// Subtract `b` from `a` and store the result in `dest`.
  sub <dest> <a> <b>,
  /// Print a value in a register.
  print <reg>,
  /// Return from the current function call.
  ret,
}

pub struct Chunk<Value: Hash + Eq> {
  pub name: Cow<'static, str>,
  pub bytecode: BytecodeArray,
  /// Pool of constants referenced in the bytecode.
  pub const_pool: Vec<Value>,
}

impl<Value: std::fmt::Display + Hash + Eq> Chunk<Value> {
  pub fn disassemble(&self) -> String {
    use std::fmt::Write;

    let mut output = String::new();

    // name
    writeln!(&mut output, "function <{}>:", self.name).unwrap();
    writeln!(&mut output, "length = {}", self.bytecode.len()).unwrap();

    // constants
    if self.const_pool.is_empty() {
      writeln!(&mut output, "const pool = <empty>").unwrap();
    } else {
      writeln!(
        &mut output,
        "const pool = (length={}) {{",
        self.const_pool.len()
      )
      .unwrap();
      for (i, value) in self.const_pool.iter().enumerate() {
        writeln!(&mut output, "  {i} = {value}").unwrap();
      }
      writeln!(&mut output, "}}").unwrap();
    }

    // bytecode
    writeln!(&mut output, "bytecode:").unwrap();
    let offset_align = self.bytecode.len().to_string().len();
    let mut pc = 0;
    while pc < self.bytecode.len() {
      let instr = self.bytecode.disassemble(pc).unwrap();
      let size = instr.size();

      write!(&mut output, " {pc:offset_align$} | ").unwrap();

      let mut bytes = self.bytecode.inner[pc..pc + size].iter().peekable();
      while let Some(byte) = bytes.next() {
        write!(&mut output, "{byte:02x}").unwrap();
        if bytes.peek().is_some() {
          write!(&mut output, " ").unwrap();
        }
      }
      if size < 6 {
        for _ in 0..(6 - size) {
          write!(&mut output, "   ").unwrap();
        }
      }
      writeln!(&mut output, " {instr}").unwrap();

      pc += size;
    }
    output
  }
}

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

      bytecode: BytecodeArray { inner: Vec::new() },
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
    if self.bytecode.fetch(self.bytecode.len() - 1) != op::__suspend {
      self.op_suspend();
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

fn patch_jumps(function_name: &str, bytecode: &mut BytecodeArray, label_map: &HashMap<u32, Label>) {
  let mut used_labels = HashSet::new();
  for pc in 0..bytecode.len() {
    let op = bytecode.fetch(pc);
    match op {
      op if is_jump_op(op) => {
        // all jump instructions are emitted with `xwide` prefix by default,
        // then narrowed based on the final offset value

        let [label_id] = bytecode.get_args(op, pc, Width::_4);
        let label = label_map
          .get(&label_id)
          .unwrap_or_else(|| panic!("unknown label ID {label_id}"));
        let jump_offset = label
          .offset
          .unwrap_or_else(|| panic!("unfinished label `{}` ({})", label.name, label.id));
        used_labels.insert(label.clone());

        // pc = offset of width prefix
        patch_jump_op(bytecode.get_buffer_mut(), op, pc - 1, jump_offset);
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
    panic!("bytecode in functon {function_name} had some unused labels (see above)");
  }
}

pub struct BytecodeArray {
  inner: Vec<u8>,
}

impl BytecodeArray {
  pub fn len(&self) -> usize {
    self.inner.len()
  }

  pub fn is_empty(&self) -> bool {
    self.inner.is_empty()
  }

  pub fn fetch(&self, pc: usize) -> u8 {
    self.inner[pc]
  }

  fn get_args<const N: usize>(&self, opcode: u8, pc: usize, width: Width) -> [u32; N] {
    let start = 1 + pc;
    if start + N * width as usize >= self.inner.len() {
      panic!(
        "malformed bytecode: missing operands for opcode {opcode} (pc={pc}, w={width}, n={N})"
      );
    }

    let mut args = [0u32; N];
    for i in 0..N {
      let offset = start + i * width as usize;
      args[i] = unsafe { self._fetch_arg(offset, width) };
    }
    args
  }

  fn get_arg(&self, opcode: u8, pc: usize, i: usize, width: Width) -> u32 {
    let offset = 1 + pc + i * width as usize;
    if offset + width as usize >= self.inner.len() {
      panic!("malformed bytecode: missing operand for opcode {opcode} (pc={pc}, w={width}, i={i})");
    }
    unsafe { self._fetch_arg(offset, width) }
  }

  unsafe fn _fetch_arg(&self, offset: usize, width: Width) -> u32 {
    match width {
      Width::_1 => unsafe { *self.inner.get_unchecked(offset) as u32 },
      Width::_2 => unsafe {
        u16::from_le_bytes([
          *self.inner.get_unchecked(offset),
          *self.inner.get_unchecked(offset + 1),
        ]) as u32
      },
      Width::_4 => unsafe {
        u32::from_le_bytes([
          *self.inner.get_unchecked(offset),
          *self.inner.get_unchecked(offset + 1),
          *self.inner.get_unchecked(offset + 2),
          *self.inner.get_unchecked(offset + 3),
        ])
      },
    }
  }

  pub fn get_buffer_mut(&mut self) -> &mut Vec<u8> {
    &mut self.inner
  }
}

#[repr(u8)]
#[derive(Clone, Copy)]
enum Width {
  _1 = 1,
  _2 = 2,
  _4 = 4,
}

impl std::fmt::Display for Width {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}",
      match self {
        Width::_1 => "1",
        Width::_2 => "2",
        Width::_4 => "4",
      }
    )
  }
}

pub enum Jump {
  /// Go to `offset`.
  Goto { offset: u32 },
  /// Skip this jump instruction.
  Skip,
}

#[cfg(test)]
mod tests;