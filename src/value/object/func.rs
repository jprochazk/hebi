use std::fmt::Display;
use std::ptr::NonNull;

use indexmap::IndexMap;

use super::module::ModuleId;
use super::{Access, List, Str};
use crate::ctx::Context;
use crate::op;
use crate::value::constant::Constant;
use crate::value::handle::Handle;
use crate::value::Value;

pub struct FunctionDescriptor {
  name: Handle<Str>,
  frame_size: u32,
  code: NonNull<[u8]>,
  const_pool: NonNull<[Constant]>,
  params: Params,
  num_captures: u32,
}

// Why are we manually managing the `code` and `const_pool` memory here?
// Dynamic languages are slow because every access to every value needs
// to be type checked.
// To mitigate this problem, modern VMs employ techniques such as inline
// caching. The basic idea is to store fields of objects tightly packed in an
// array or tuple-like structure, and then after an initial lookup by name,
// cache the index of the field in the bytecode. As long as the shape of
// the object doesn't changes, every access to that field will be significantly
// faster.
// Note the `cache the index` part. This requires mutating the bytecode as it's
// running, meaning we need read *and* write access to it at the same time. This
// is already not very in line with Rust's borrow checking rules, but we can
// just use RefCell, right? Consider the case of recursive functions. Each new
// frame on the call stack needs to store a reference to the bytecode, so that
// the VM can access it with minimal indirection when dispatching instructions.
// The moment we try to read an instruction (immutable borrow), followed by an
// attempt to IC a field or quicken the instruction (mutable borrow), we get
// a panic.
// There is no way to express this pattern in safe Rust today, so we fall back
// to using raw pointers. This way we side-step Rust's aliasing rules, and can
// store as many mutable pointers to the same memory as we'd like, without
// invoking UB.
// Can't we at least use a `Box` or a `Vec`?
// No, `Box` is not viable here, because there is no easy way to
// obtain a `NonNull` pointer from a `Box` without consuming the `Box`.
// Same problem for `Vec`. They want to own the memory, and giving
// you a raw pointer means opening you up to dangling pointers and
// use-after-free. Armed with miri and valgrind, the risk is worth it here.

impl Drop for FunctionDescriptor {
  fn drop(&mut self) {
    drop(unsafe { Box::from_raw(self.code.as_ptr()) });
    drop(unsafe { Box::from_raw(self.const_pool.as_ptr()) });
  }
}

impl FunctionDescriptor {
  pub fn new(
    name: Handle<Str>,
    frame_size: u32,
    code: Vec<u8>,
    const_pool: Vec<Constant>,
    params: Params,
    num_captures: u32,
  ) -> Self {
    let code = unsafe { NonNull::new_unchecked(Box::into_raw(code.into_boxed_slice())) };
    let const_pool =
      unsafe { NonNull::new_unchecked(Box::into_raw(const_pool.into_boxed_slice())) };
    Self {
      name,
      frame_size,
      code,
      const_pool,
      params,
      num_captures,
    }
  }
}

#[derive::delegate_to_handle]
impl FunctionDescriptor {
  pub fn name(&self) -> Handle<Str> {
    self.name.clone()
  }

  pub fn frame_size(&self) -> u32 {
    self.frame_size
  }

  /// # Safety
  /// Caller must ensure that:
  /// - `self` is not dropped before the pointer returned by this function.
  /// - the pointer is never freed.
  /// - any references constructed from the pointer are short-lived and adhere
  ///   to Rust's aliasing guarantees.
  pub unsafe fn code_mut(&mut self) -> NonNull<[u8]> {
    self.code
  }

  /// # Safety
  /// Caller must ensure that:
  /// - `self` is not dropped before the pointer returned by this function.
  /// - the pointer is never freed.
  /// - any references constructed from the pointer are short-lived and adhere
  ///   to Rust's aliasing guarantees.
  pub unsafe fn const_pool(&self) -> NonNull<[Constant]> {
    self.const_pool
  }

  pub fn params(&self) -> &Params {
    &self.params
  }

  pub fn num_captures(&self) -> u32 {
    self.num_captures
  }

  pub fn disassemble(&self, print_bytes: bool) -> String {
    let mut out = String::new();

    self._disassemble_inner(&mut out, print_bytes);
    out
  }

  pub(crate) fn _disassemble_inner(&self, f: &mut String, print_bytes: bool) {
    let const_pool = unsafe { self.const_pool.as_ref() };
    for v in const_pool.iter() {
      if let Constant::FunctionDescriptor(func) = v {
        func._disassemble_inner(f, print_bytes);
        f.push('\n');
      }
    }

    use std::fmt::Write;

    // name
    writeln!(f, "function {}:", self.name).unwrap();
    writeln!(f, "  frame_size: {}", self.frame_size).unwrap();
    writeln!(f, "  length: {}", self.code.len()).unwrap();

    // constants
    if const_pool.is_empty() {
      writeln!(f, "  const: <empty>").unwrap();
    } else {
      writeln!(f, "  const (length={}):", const_pool.len()).unwrap();
      for (i, value) in const_pool.iter().enumerate() {
        writeln!(f, "    {i}: {value}").unwrap();
      }
    }

    // bytecode
    writeln!(f, "  code:").unwrap();
    let offset_align = self.code.len().to_string().len();
    let mut pc = 0;
    while pc < self.code.len() {
      let code = unsafe { self.code.as_ref() };
      let (size, instr) = op::disassemble(code, pc);

      let bytes = {
        let mut out = String::new();
        if print_bytes {
          for byte in code[pc..pc + size].iter() {
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

impl Display for FunctionDescriptor {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<function descriptor {}>", self.name())
  }
}

impl Access for FunctionDescriptor {}

pub struct Function {
  desc: Handle<FunctionDescriptor>,
  captures: Handle<List>,
  module_id: Option<ModuleId>,
}

impl Function {
  pub fn new(
    ctx: &Context,
    desc: Handle<FunctionDescriptor>,
    module_id: Option<ModuleId>,
  ) -> Handle<Function> {
    let captures = ctx.alloc(List::from_iter(
      (0..desc.num_captures() as usize).map(|_| Value::none()),
    ));
    ctx.alloc(Function {
      desc,
      captures,
      module_id,
    })
  }
}

#[derive::delegate_to_handle]
impl Function {
  pub fn descriptor(&self) -> Handle<FunctionDescriptor> {
    self.desc.clone()
  }

  pub fn module_id(&self) -> Option<ModuleId> {
    self.module_id
  }

  pub fn captures(&self) -> Handle<List> {
    self.captures.clone()
  }
}

impl Display for Function {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<function {}>", self.descriptor().name())
  }
}

impl Access for Function {}

pub struct Params {
  pub has_self: bool,
  pub min: usize,
  pub max: usize,
  pub argv: bool,
  pub kwargs: bool,
  pub pos: Vec<String>,
  pub kw: IndexMap<String, bool>,
}

pub fn name(f: &Value) -> Handle<Str> {
  if let Some(f) = f.clone().to_function() {
    f.descriptor().name()
  } else if let Some(f) = f.clone().to_native_function() {
    f.name()
  } else if let Some(f) = f.clone().to_native_class_method() {
    f.name()
  } else if let Some(f) = f.clone().to_method() {
    name(&f.func())
  } else {
    panic!("not a function: {f}")
  }
}

/// Object types which may be called directly
pub fn is_callable(f: &Value) -> bool {
  f.is_function() || f.is_native_function() || f.is_method() || f.is_native_class_method()
}
