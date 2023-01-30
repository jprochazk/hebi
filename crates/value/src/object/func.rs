use beef::lean::Cow;
use indexmap::IndexMap;

use crate::ptr::{Ref, RefMut};
use crate::Value;

#[derive(Clone, Debug)]
pub struct Func {
  pub(super) name: Cow<'static, str>,
  pub(super) frame_size: u32,
  pub(super) code: Vec<u8>,
  pub(super) const_pool: Vec<Value>,
  pub(super) params: Params,
}

#[derive(Clone, Debug)]
pub struct ClosureDescriptor {
  pub func: Func,
  pub num_captures: u32,
}

#[derive(Clone, Debug)]
pub struct Closure {
  pub(super) descriptor: Value,
  pub captures: Vec<Value>,
}

impl Closure {
  /// Create a new closure.
  ///
  /// # Panics
  /// If `func.is_closure_descriptor()` is not `true`.
  pub fn new(descriptor: Value) -> Self {
    // TODO: can this be encoded via the type system?
    assert!(
      descriptor.is_closure_descriptor(),
      "closure may only be constructed from a closure descriptor"
    );

    let captures = {
      let descriptor = unsafe { descriptor.as_closure_descriptor().unwrap_unchecked() };
      let mut v = Vec::with_capacity(descriptor.num_captures as usize);
      for _ in 0..descriptor.num_captures {
        v.push(Value::none());
      }
      v
    };

    Self {
      descriptor,
      captures,
    }
  }

  fn func(&self) -> Ref<'_, Func> {
    Ref::map(
      unsafe { self.descriptor.as_closure_descriptor().unwrap_unchecked() },
      |v| &v.func,
    )
  }

  fn func_mut(&mut self) -> RefMut<'_, Func> {
    RefMut::map(
      unsafe {
        self
          .descriptor
          .as_closure_descriptor_mut()
          .unwrap_unchecked()
      },
      |v| &mut v.func,
    )
  }

  pub fn name(&self) -> Ref<'_, str> {
    Ref::map(self.func(), |v| v.name.as_ref())
  }

  pub fn frame_size(&self) -> u32 {
    self.func().frame_size
  }

  pub fn code(&self) -> Ref<'_, [u8]> {
    Ref::map(self.func(), |v| &v.code[..])
  }

  pub fn code_mut(&mut self) -> RefMut<'_, [u8]> {
    RefMut::map(self.func_mut(), |v| &mut v.code[..])
  }

  pub fn const_pool(&self) -> Ref<'_, [Value]> {
    Ref::map(self.func(), |v| &v.const_pool[..])
  }

  pub fn params(&self) -> Ref<'_, Params> {
    Ref::map(self.func(), |v| &v.params)
  }

  pub fn disassemble<D>(
    &self,
    disassemble_instruction: fn(&[u8], usize) -> (usize, D),
    print_bytes: bool,
  ) -> String
  where
    D: std::fmt::Display,
  {
    self
      .func()
      .disassemble(disassemble_instruction, print_bytes)
  }
}

#[derive(Clone, Debug)]
pub struct Params {
  pub min: usize,
  pub max: usize,
  pub argv: bool,
  pub kwargs: bool,
  pub pos: Vec<String>,
  pub kw: IndexMap<String, bool>,
}

impl Func {
  pub fn new(
    name: Cow<'static, str>,
    frame_size: u32,
    code: Vec<u8>,
    const_pool: Vec<Value>,
    params: Params,
  ) -> Self {
    Self {
      name,
      frame_size,
      code,
      const_pool,
      params,
    }
  }

  pub fn name(&self) -> &str {
    self.name.as_ref()
  }

  pub fn frame_size(&self) -> u32 {
    self.frame_size
  }

  pub fn code(&self) -> &[u8] {
    &self.code
  }

  pub fn code_mut(&mut self) -> &mut [u8] {
    &mut self.code
  }

  pub fn const_pool(&self) -> &[Value] {
    &self.const_pool
  }

  pub fn params(&self) -> &Params {
    &self.params
  }

  pub fn disassemble<D>(
    &self,
    disassemble_instruction: fn(&[u8], usize) -> (usize, D),
    print_bytes: bool,
  ) -> String
  where
    D: std::fmt::Display,
  {
    let mut out = String::new();

    self.disassemble_inner(disassemble_instruction, &mut out, print_bytes);
    out
  }

  fn disassemble_inner<D>(
    &self,
    disassemble_instruction: fn(&[u8], usize) -> (usize, D),
    f: &mut String,
    print_bytes: bool,
  ) where
    D: std::fmt::Display,
  {
    for v in self.const_pool.iter() {
      if let Some(func) = v.as_func() {
        func.disassemble_inner(disassemble_instruction, f, print_bytes);
        f.push('\n');
      } else if let Some(descriptor) = v.as_closure_descriptor() {
        descriptor
          .func
          .disassemble_inner(disassemble_instruction, f, print_bytes);
        f.push('\n');
      }
    }

    use std::fmt::Write;

    // name
    writeln!(f, "function <{}>:", self.name).unwrap();
    writeln!(f, "  frame_size: {}", self.frame_size).unwrap();
    writeln!(f, "  length: {}", self.code.len()).unwrap();

    // constants
    if self.const_pool.is_empty() {
      writeln!(f, "  const: <empty>").unwrap();
    } else {
      writeln!(f, "  const (length={}):", self.const_pool.len()).unwrap();
      for (i, value) in self.const_pool.iter().enumerate() {
        writeln!(f, "    {i}: {value}").unwrap();
      }
    }

    // bytecode
    writeln!(f, "  code:").unwrap();
    let offset_align = self.code.len().to_string().len();
    let mut pc = 0;
    while pc < self.code.len() {
      let (size, instr) = disassemble_instruction(&self.code[..], pc);

      let bytes = {
        let mut out = String::new();
        if print_bytes {
          for byte in self.code[pc..pc + size].iter() {
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
}
