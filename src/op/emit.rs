// TEMP
#![allow(dead_code)]

mod regalloc;

use beef::lean::Cow;
use indexmap::{IndexMap, IndexSet};

use self::regalloc::{RegAlloc, Register};
use super::Instruction;
use crate::ctx::Context;
use crate::error::Result;
use crate::op;
use crate::syntax::ast;
use crate::value::constant::Constant;
use crate::value::object;
use crate::value::object::ptr::Ptr;

pub fn emit<'cx, 'src>(
  cx: &'cx Context,
  ast: &'src ast::Module<'src>,
  name: impl Into<Cow<'src, str>>,
  is_root: bool,
) -> Result<Ptr<object::Module>> {
  State::new(cx, ast, name, is_root).emit()
}

struct State<'cx, 'src> {
  cx: &'cx Context,
  ast: &'src ast::Module<'src>,
  module: Module<'src>,
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
        functions: vec![Function::new(name)],
      },
    }
  }

  fn emit(self) -> Result<Ptr<object::Module>> {
    todo!()
  }
}

struct Module<'src> {
  is_root: bool,
  vars: IndexSet<Ptr<String>>,
  functions: Vec<Function<'src>>,
}

struct Function<'src> {
  name: Cow<'src, str>,
  instructions: Vec<Instruction>,
  constants: IndexSet<Constant>,
  regalloc: RegAlloc,

  locals: IndexMap<(Scope, Cow<'src, str>), Register>,
  upvalues: IndexMap<Cow<'src, str>, Upvalue>,

  is_in_opt_expr: bool,
  current_loop: Option<Loop>,
}

impl<'src> Function<'src> {
  fn new(name: impl Into<Cow<'src, str>>) -> Self {
    Self {
      name: name.into(),
      instructions: Vec::new(),
      constants: IndexSet::new(),
      regalloc: RegAlloc::new(),

      locals: IndexMap::new(),
      upvalues: IndexMap::new(),

      is_in_opt_expr: false,
      current_loop: None,
    }
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
