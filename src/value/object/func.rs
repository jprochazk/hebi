use std::fmt::Display;

use indexmap::IndexMap;

use super::{Access, Str};
use crate::op;
use crate::value::constant::Constant;
use crate::value::handle::Handle;
use crate::value::Value;

pub struct Func {
  name: Handle<Str>,
  frame_size: u32,
  code: Vec<u8>,
  const_pool: Vec<Constant>,
  params: Params,
}

impl Func {
  pub fn new(
    name: Handle<Str>,
    frame_size: u32,
    code: Vec<u8>,
    const_pool: Vec<Constant>,
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
}

#[derive::delegate_to_handle]
impl Func {
  pub fn name(&self) -> Handle<Str> {
    self.name.clone()
  }

  pub fn frame_size(&self) -> u32 {
    self.frame_size
  }

  pub fn code(&self) -> &[u8] {
    &self.code
  }

  pub unsafe fn code_mut(&mut self) -> &mut [u8] {
    &mut self.code
  }

  pub fn const_pool(&self) -> &[Constant] {
    &self.const_pool
  }

  pub fn params(&self) -> &Params {
    &self.params
  }

  pub fn disassemble(&self, print_bytes: bool) -> String {
    let mut out = String::new();

    self.disassemble_inner(&mut out, print_bytes);
    out
  }

  fn disassemble_inner(&self, f: &mut String, print_bytes: bool) {
    for v in self.const_pool.iter() {
      if let Constant::Func(func) = v {
        unsafe { func._get() }.disassemble_inner(f, print_bytes);
        f.push('\n');
      } else if let Constant::ClosureDesc(desc) = v {
        unsafe { desc.func()._get() }.disassemble_inner(f, print_bytes);
        f.push('\n');
      }
    }

    use std::fmt::Write;

    // name
    writeln!(f, "function {}:", self.name).unwrap();
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
      let (size, instr) = op::disassemble(&self.code[..], pc);

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

impl Display for Func {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<func {}>", self.name())
  }
}

impl Access for Func {}

pub struct ClosureDesc {
  func: Handle<Func>,
  num_captures: u32,
}

impl ClosureDesc {
  pub fn new(func: Handle<Func>, num_captures: u32) -> Self {
    Self { func, num_captures }
  }
}

#[derive::delegate_to_handle]
impl ClosureDesc {
  pub fn func(&self) -> Handle<Func> {
    self.func.clone()
  }

  pub fn num_captures(&self) -> u32 {
    self.num_captures
  }
}

impl Display for ClosureDesc {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<closure desc {}>", self.func().name())
  }
}

impl Access for ClosureDesc {}

pub struct Closure {
  descriptor: Handle<ClosureDesc>,
  captures: Vec<Value>,
}

impl Closure {
  /// Create a new closure.
  pub fn new(descriptor: Handle<ClosureDesc>) -> Self {
    let captures = {
      let mut v = Vec::with_capacity(descriptor.num_captures() as usize);
      for _ in 0..descriptor.num_captures() {
        v.push(Value::none());
      }
      v
    };

    Self {
      descriptor,
      captures,
    }
  }
}

#[derive::delegate_to_handle]
impl Closure {
  pub fn descriptor(&self) -> Handle<ClosureDesc> {
    self.descriptor.clone()
  }

  /* pub fn captures(&self) -> &[Value] {
    &self.captures
  } */

  pub unsafe fn captures_mut(&mut self) -> &mut [Value] {
    &mut self.captures
  }
}

impl Display for Closure {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<closure {}>", self.descriptor().func().name())
  }
}

impl Access for Closure {}

pub struct Params {
  pub has_self: bool,
  pub min: usize,
  pub max: usize,
  pub argv: bool,
  pub kwargs: bool,
  pub pos: Vec<String>,
  pub kw: IndexMap<String, bool>,
}

pub fn func_name(f: &Value) -> String {
  if let Some(f) = f.clone().to_func() {
    f.name().as_str().to_string()
  } else if let Some(f) = f.clone().to_closure() {
    f.descriptor().func().name().as_str().to_string()
  } else if let Some(f) = f.clone().to_method() {
    func_name(&f.func())
  } else {
    panic!("{f} is not callable")
  }
}
