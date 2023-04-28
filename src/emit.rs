// TEMP
#![allow(dead_code)]

mod regalloc;
mod stmt;

// TODO:
// - (optimization) constant pool compaction
// - (optimization) basic blocks
// - (optimization) elide last instruction (clobbered read)
// - (optimization) peephole with last 2 instructions
// - register allocation
// - actually write emit for all AST nodes

use beef::lean::Cow;
use indexmap::{IndexMap, IndexSet};

use self::regalloc::{RegAlloc, Register};
use crate::bytecode::builder::BytecodeBuilder;
use crate::bytecode::opcode::symbolic::*;
use crate::bytecode::opcode::{self as op};
use crate::ctx::Context;
use crate::syntax::ast;
use crate::value::object;
use crate::value::object::function;
use crate::value::object::ptr::Ptr;

pub fn emit<'cx, 'src>(
  cx: &'cx Context,
  ast: &'src ast::Module<'src>,
  name: impl Into<Cow<'src, str>>,
  is_root: bool,
) -> Ptr<object::ModuleDescriptor> {
  let name = name.into();

  let mut module = State::new(cx, ast, name.clone(), is_root).emit_module();

  let name = cx.alloc(object::String::new(name.to_string().into()));
  let root = module.functions.pop().unwrap().finish();
  let module_vars = module.vars;

  cx.alloc(object::ModuleDescriptor {
    name,
    root,
    module_vars,
  })
}

struct State<'cx, 'src> {
  cx: &'cx Context,
  ast: &'src ast::Module<'src>,
  module: Module<'cx, 'src>,
}

impl<'cx, 'src> State<'cx, 'src> {
  fn new(
    cx: &'cx Context,
    ast: &'src ast::Module<'src>,
    name: impl Into<Cow<'src, str>>,
    is_root: bool,
  ) -> Self {
    Self {
      cx,
      ast,
      module: Module {
        is_root,
        vars: IndexSet::new(),
        functions: vec![Function::new(cx, name, function::Params::default())],
      },
    }
  }

  fn current_function(&mut self) -> &mut Function<'cx, 'src> {
    self.module.functions.last_mut().unwrap()
  }

  fn builder(&mut self) -> &mut BytecodeBuilder {
    &mut self.current_function().builder
  }

  fn emit_module(mut self) -> Module<'cx, 'src> {
    for stmt in self.ast.body.iter() {
      self.emit_stmt(stmt);
    }
    self.builder().emit(Ret, 0..0);

    self.module
  }
}

struct Module<'cx, 'src> {
  is_root: bool,
  vars: IndexSet<Ptr<object::String>>,
  functions: Vec<Function<'cx, 'src>>,
}

struct Function<'cx, 'src> {
  cx: &'cx Context,

  name: Cow<'src, str>,
  builder: BytecodeBuilder,
  regalloc: RegAlloc,

  params: function::Params,
  locals: IndexMap<(Scope, Cow<'src, str>), Register>,
  upvalues: IndexMap<Cow<'src, str>, Upvalue>,

  is_in_opt_expr: bool,
  current_loop: Option<Loop>,
}

impl<'cx, 'src> Function<'cx, 'src> {
  fn new(cx: &'cx Context, name: impl Into<Cow<'src, str>>, params: function::Params) -> Self {
    Self {
      cx,

      name: name.into(),
      builder: BytecodeBuilder::new(),
      regalloc: RegAlloc::new(),

      params,
      locals: IndexMap::new(),
      upvalues: IndexMap::new(),

      is_in_opt_expr: false,
      current_loop: None,
    }
  }

  fn finish(self) -> Ptr<object::FunctionDescriptor> {
    let (frame_size, register_map) = self.regalloc.finish();
    let (mut bytecode, constants) = self.builder.finish();
    op::patch_registers(&mut bytecode, &register_map);

    self.cx.alloc(object::FunctionDescriptor::new(
      self
        .cx
        .alloc(object::String::new(self.name.to_string().into())),
      self.params,
      self.upvalues.len(),
      frame_size,
      bytecode,
      constants,
    ))
  }
}

enum Upvalue {
  /// Upvalue a local in the outer scope
  Parent { src: Register, dst: op::Upvalue },
  /// Upvalue an upvalue in the outer scope
  Nested { src: op::Upvalue, dst: op::Upvalue },
}

struct Loop {
  start: Label,
  end: Label,
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Label(usize);

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Scope(usize);

#[cfg(test)]
mod tests;
