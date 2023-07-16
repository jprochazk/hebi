#![allow(clippy::needless_lifetimes)]

mod builder;

use alloc::format;
use alloc::string::String;
use core::cmp::max;
use core::fmt::{Debug, Display};
use core::hash::BuildHasherDefault;

use bumpalo::collections::{CollectionAllocErr, Vec};
use bumpalo::AllocErr;
use rustc_hash::FxHasher;

use self::builder::BytecodeBuilder;
use super::{Mvar, Op, Reg, Upvalue};
use crate::ast::{Block, Expr, Func, GetVar, Let, Lit, Loop, Module, Return, Stmt};
use crate::error::StdError;
use crate::gc::{Gc, Ref};
use crate::lex::Span;
use crate::obj::func::{Code, FunctionDescriptor, Params};
use crate::obj::module::ModuleDescriptor;
use crate::op::asm::*;
use crate::op::Smi;
use crate::{alloc, Arena};

pub type Result<T> = core::result::Result<T, EmitError>;

#[derive(Debug)]
pub struct EmitError {
  pub message: String,
}

impl EmitError {
  pub fn new(message: impl Into<String>) -> EmitError {
    EmitError {
      message: message.into(),
    }
  }
}

impl From<CollectionAllocErr> for EmitError {
  fn from(e: CollectionAllocErr) -> Self {
    match e {
      CollectionAllocErr::CapacityOverflow => Self::new(String::from("capacity overflow")),
      CollectionAllocErr::AllocErr => Self::new(format!("{}", AllocErr)),
    }
  }
}

impl From<AllocErr> for EmitError {
  fn from(e: AllocErr) -> Self {
    Self::new(format!("{}", e))
  }
}

impl Display for EmitError {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "error: {}", self.message)
  }
}

impl StdError for EmitError {}

type HashSet<T, A> = hashbrown::HashSet<T, BuildHasherDefault<FxHasher>, A>;
type HashMap<K, V, A> = hashbrown::HashMap<K, V, BuildHasherDefault<FxHasher>, A>;

struct ModuleState<'arena, 'gc, 'src> {
  arena: &'arena Arena,
  gc: &'gc Gc,
  name: &'src str,
  ast: Module<'arena, 'src>,

  /// This is a map of top-level variables, a.k.a. global variables.
  /// In hebi they're referred to as "module" variables, because
  /// they're instantiated per module.
  vars: HashMap<&'src str, Mvar<u16>, &'arena Arena>,
}

struct FunctionState<'arena, 'gc, 'src, 'state> {
  module: &'state mut ModuleState<'arena, 'gc, 'src>,
  parent: Option<&'state mut FunctionState<'arena, 'gc, 'src, 'state>>,

  arena: &'arena Arena,
  gc: &'gc Gc,
  name: &'src str,

  builder: BytecodeBuilder<'arena>,
  registers: Registers,

  scope: Scope,
  locals: Vec<'arena, (Scope, &'src str, Reg<u8>)>,
}

#[derive(Clone, Copy, Default)]
struct Registers {
  current: u8,
  total: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Scope(usize);

impl<'arena, 'gc, 'src, 'state> FunctionState<'arena, 'gc, 'src, 'state> {
  #[inline]
  fn scope<F, T>(&mut self, f: F) -> Result<T>
  where
    F: FnOnce(&mut FunctionState<'arena, 'gc, 'src, 'state>) -> Result<T>,
  {
    self.scope.0 += 1;
    let result = f(self);
    self.scope.0 -= 1;
    result
  }

  #[doc(hidden)]
  #[inline]
  fn _reg(&mut self) -> Reg<u8> {
    let reg = self.registers.current;
    self.registers.current += 1;
    self.registers.total = max(self.registers.current, self.registers.total);
    Reg(reg)
  }

  #[inline]
  fn reg(&mut self) -> Result<Reg<u8>> {
    if self.registers.current == u8::MAX {
      return Err(EmitError::new(format!(
        "function `{}` uses too many registers, maximum is {}",
        self.name,
        u8::MAX
      )));
    }
    Ok(self._reg())
  }

  #[inline]
  fn free(&mut self, r: Reg<u8>) {
    self.registers.current = r.0;
  }

  #[inline]
  fn emit(&mut self, op: Op, span: impl Into<Span>) -> Result<()> {
    self.builder.emit(op, span)?;
    Ok(())
  }

  #[inline]
  fn is_top_level(&self) -> bool {
    self.parent.is_none() && self.scope.0 <= 1
  }

  /// Invariant: `reg` must already contain the value
  ///
  /// Note: This frees `reg` if necessary
  fn declare_var(&mut self, name: &'src str, reg: Reg<u8>, span: impl Into<Span>) -> Result<()> {
    if self.is_top_level() {
      // module variable
      // value is in `reg`, we have to add the var to module.vars
      let last = self.module.vars.len();
      if last > u16::MAX as usize {
        return Err(EmitError::new(format!(
          "too many global variables, maximum is {}",
          u16::MAX
        )));
      }
      let last = last as u16;
      // if the var already exists, reuse it (as the previous one was shadowed)
      // this means:
      //   let a = 0
      //   let a = 0
      // is the same as:
      //   let a = 0
      //   a = 0
      let idx = *self.module.vars.entry(name).or_insert_with(|| Mvar(last));
      self.emit(set_mvar(reg, idx), span)?;
      self.free(reg);
    } else {
      // local variable
      // no need to emit anything, just add it to locals
      if !self
        .locals
        .iter()
        .any(|(scope, name0, _)| (scope, *name0) == (&self.scope, name))
      {
        self.locals.push((self.scope, name, reg));
      }
      // note: doing nothing is fine if `locals` already contains
      // `(scope, name)`, `reg` is already reusing an existing register
      // if possible, and it's already set to the correct value.
    }

    Ok(())
  }

  fn resolve_var(&self, name: &'src str) -> Var {
    if self.is_top_level() {
      if let Some(reg) = self.resolve_local(name) {
        Var::Local(reg)
      } else if let Some(idx) = self.module.vars.get(name).copied() {
        Var::Module(idx)
      } else {
        Var::Global
      }
    } else if let Some(reg) = self.resolve_local(name) {
      Var::Local(reg)
    } else if let Some(idx) = self.resolve_upvalue(name) {
      Var::Upvalue(idx)
    } else if let Some(idx) = self.module.vars.get(name).copied() {
      Var::Module(idx)
    } else {
      Var::Global
    }
  }

  fn resolve_local(&self, name: &'src str) -> Option<Reg<u8>> {
    self
      .locals
      .iter()
      .rfind(|(_, var, _)| *var == name)
      .map(|(_, _, register)| *register)
  }

  fn resolve_local_in_scope(&self, scope: Scope, name: &'src str) -> Option<Reg<u8>> {
    self
      .locals
      .iter()
      .rfind(|(scope0, var, _)| (scope0, *var) == (&scope, name))
      .map(|(_, _, register)| *register)
  }

  fn resolve_upvalue(&self, name: &'src str) -> Option<Upvalue<u16>> {
    todo!()
  }
}

pub fn module<'arena, 'gc, 'src>(
  arena: &'arena Arena,
  gc: &'gc Gc,
  name: &'src str,
  ast: Module<'arena, 'src>,
) -> Result<Ref<ModuleDescriptor>> {
  let mut module = ModuleState {
    arena,
    gc,
    name,
    ast,

    vars: HashMap::with_hasher_in(BuildHasherDefault::default(), arena),
  };
  let root = top_level(&mut module, arena, gc)?;
  Ok(ModuleDescriptor::try_new_in(
    gc,
    name,
    root,
    module.vars.len() as u16,
  )?)
}

fn top_level<'arena, 'gc, 'src>(
  module: &mut ModuleState<'arena, 'gc, 'src>,
  arena: &'arena Arena,
  gc: &'gc Gc,
) -> Result<Ref<FunctionDescriptor>> {
  let mut f = FunctionState {
    module,
    parent: None,

    arena,
    gc,
    name: "__main__",

    builder: BytecodeBuilder::new_in(arena),
    registers: Registers::default(),
    scope: Scope(0),
    locals: Vec::new_in(arena),
  };

  f.scope(|f| {
    for node in f.module.ast {
      stmt(f, node)?;
    }
    Ok(())
  })?;

  f.emit(finalize_module(), Span::empty())?;
  f.free(Reg(0));
  let dst = f.reg()?;
  f.emit(load_nil(dst), Span::empty())?;
  f.emit(ret(dst), Span::empty())?;

  let stack_space = f.registers.total;
  let (ops, pool, loc) = f.builder.finish();
  let code = Code {
    stack_space,
    ops: &ops,
    pool: &pool,
    loc: &loc,
  };

  Ok(FunctionDescriptor::try_new_in(
    gc,
    f.name,
    Params::empty(),
    code,
  )?)
}

fn stmt<'arena, 'gc, 'src, 'state>(
  f: &mut FunctionState<'arena, 'gc, 'src, 'state>,
  node: &Stmt<'arena, 'src>,
) -> Result<()> {
  use crate::ast::StmtKind::*;

  match node.kind {
    Let(inner) => let_(f, inner, node.span),
    Loop(inner) => loop_(f, inner),
    Break => break_(f),
    Continue => continue_(f),
    Return(inner) => return_(f, inner),
    Func(inner) => func(f, inner),
    Expr(inner) => {
      let _ = expr(f, None, inner)?;
      Ok(())
    }
  }
}

fn let_<'arena, 'gc, 'src, 'state>(
  f: &mut FunctionState<'arena, 'gc, 'src, 'state>,
  node: &Let<'arena, 'src>,
  span: Span,
) -> Result<()> {
  // note: `declare_var` at the end is responsible for freeing `dst` if necessary
  let dst = match f.resolve_local_in_scope(f.scope, node.name.lexeme) {
    Some(reg) => reg,
    None => f.reg()?,
  };

  if let Some(value) = &node.value {
    if let Some(out) = expr(f, Some(dst), value)? {
      // `expr` was written to `out`
      f.emit(mov(out, dst), value.span)?;
    } else {
      // `expr` was written to `dst`
    }
  } else {
    f.emit(load_nil(dst), span)?;
  }

  f.declare_var(node.name.lexeme, dst, span)?;

  Ok(())
}

fn loop_<'arena, 'gc, 'src, 'state>(
  f: &mut FunctionState<'arena, 'gc, 'src, 'state>,
  node: &Loop,
) -> Result<()> {
  todo!()
}

fn break_<'arena, 'gc, 'src, 'state>(
  f: &mut FunctionState<'arena, 'gc, 'src, 'state>,
) -> Result<()> {
  todo!()
}

fn continue_<'arena, 'gc, 'src, 'state>(
  f: &mut FunctionState<'arena, 'gc, 'src, 'state>,
) -> Result<()> {
  todo!()
}

fn return_<'arena, 'gc, 'src, 'state>(
  f: &mut FunctionState<'arena, 'gc, 'src, 'state>,
  node: &Return,
) -> Result<()> {
  todo!()
}

fn func<'arena, 'gc, 'src, 'state>(
  f: &mut FunctionState<'arena, 'gc, 'src, 'state>,
  node: &Func,
) -> Result<()> {
  todo!()
}

fn expr<'arena, 'gc, 'src, 'state>(
  f: &mut FunctionState<'arena, 'gc, 'src, 'state>,
  dst: Option<Reg<u8>>,
  node: &Expr<'arena, 'src>,
) -> Result<Option<Reg<u8>>> {
  use crate::ast::ExprKind::*;

  match node.kind {
    Binary(_) => todo!(),
    Unary(_) => todo!(),
    Block(inner) => block(f, dst, inner),
    If(_) => todo!(),
    Func(_) => todo!(),
    GetVar(inner) => get_var(f, dst, inner, node.span),
    SetVar(_) => todo!(),
    GetField(_) => todo!(),
    SetField(_) => todo!(),
    GetIndex(_) => todo!(),
    SetIndex(_) => todo!(),
    Call(_) => todo!(),
    Lit(inner) => lit(f, dst, inner, node.span),
  }
}

fn block<'arena, 'gc, 'src, 'state>(
  f: &mut FunctionState<'arena, 'gc, 'src, 'state>,
  dst: Option<Reg<u8>>,
  node: &Block<'arena, 'src>,
) -> Result<Option<Reg<u8>>> {
  f.scope(|f| {
    for node in node.body {
      stmt(f, node)?;
    }

    match &node.last {
      Some(node) => expr(f, dst, node),
      None => Ok(None),
    }
  })
}

enum Var {
  Local(Reg<u8>),
  Upvalue(Upvalue<u16>),
  Module(Mvar<u16>),
  Global,
}

fn get_var<'arena, 'gc, 'src, 'state>(
  f: &mut FunctionState<'arena, 'gc, 'src, 'state>,
  dst: Option<Reg<u8>>,
  node: &GetVar<'src>,
  span: Span,
) -> Result<Option<Reg<u8>>> {
  use Var::*;

  match f.resolve_var(node.name.lexeme) {
    Local(reg) => Ok(Some(reg)),
    Upvalue(idx) => todo!(),
    Module(var) => {
      if let Some(dst) = dst {
        f.emit(load_mvar(dst, var), span)?;
      }
      Ok(None)
    }
    Global => {
      todo!()
    }
  }
}

fn lit<'arena, 'gc, 'src, 'state>(
  f: &mut FunctionState<'arena, 'gc, 'src, 'state>,
  dst: Option<Reg<u8>>,
  node: &Lit<'arena, 'src>,
  span: Span,
) -> Result<Option<Reg<u8>>> {
  use Lit::*;

  let Some(dst) = dst else { return Ok(None) };

  match node {
    Float(value) => todo!(),
    Int(value) => {
      if let Ok(value) = i16::try_from(*value) {
        f.emit(load_smi(dst, Smi(value)), span)?;
      } else {
        // constant + emit load const
        todo!()
      }
    }
    Nil => todo!(),
    Bool(value) => todo!(),
    String(value) => todo!(),
    Record(value) => todo!(),
    List(value) => todo!(),
    Tuple(value) => todo!(),
  }

  Ok(None)
}

/*

register allocation:

follows a stack discipline, registers are allocated as needed

- temporary registers used for intermediate results in expressions
  are freed at the end of their local scope, which is immediately
  before exiting the function which emits the expression

- variables are stored in `state.locals`, and freed upon exiting the current scope

  let v = 10 + 10
  ^ enter let_

  let v = 10 + 10
      ^ allocate register `r0`

  let v = 10 + 10
          ^ enter expr(r0)

  let v = 10 + 10
          ^ lhs emitted first, enter literal(r0)

  let v = 10 + 10
          ^ literal is directly written to `r0`
            no register is returned

  let v = 10 + 10
               ^ rhs is emitted next
                 allocate fresh register (r1)
                 enter literal(r1)

  let v = 10 + 10
               ^ literal is written to `r1`
                 no register is returned

  let v = 10 + 10
          ^ lhs and rhs are emitted,
            emit `add r1, r1, r0`
            return no register
            # Q: Is there ever a scenario where we _wouldn't_ use `r1` as `dst`?
              A: no, but even though `dst` will always be `r1`,
                 the `lhs` or `rhs` may be variables, which would mean
                 that `dst` is not always equal to `lhs`

  # exit all the way to let_

  let v = 10 + 10
      ^ received no register from `value`
        do not emit anything
        if global scope: define `v` as mvar (set_mvar(r0))
        else:            define `v` as local (add `r0` to locals)

  let u = v + 10
  ^ enter let_

  let u = v + 10
      ^ allocate register `r1`

  let u = v + 10
          ^ enter expr(r1)

  let u = v + 10
          ^ lhs first, enter get_var(r1)

  let u = v + 10
          ^ do not emit anything
            return register `r0`

  let u = v + 10
              ^ alloc fresh register (r2)
                enter expr(r2)

  let u = v + 10
              ^ enter literal(r2)

  let u = v + 10
              ^ literal is written to r2

  let u = v + 10
          ^ lhs=r0, rhs=r2
            emit(add r1, r0, r2)
            return no register

  let u = v + 10
      ^ got no register
        do not emit anything
        if global scope: define `u` as mvar (set_mvar(r1))
        else:            define `u` as local (add `r1` to locals)



  # Q: Can we use only 2 registers for the output of:
  10 + 10 + 10 + 10
  ^ expr(r0)
  ^ literal(r0)
  10 + 10 + 10 + 10
       ^ expr(r1)
       ^ literal(r1)
  10 + 10 + 10 + 10
            ^ expr(r2)
            ^ literal(r2)
  10 + 10 + 10 + 10
                 ^ expr(r3)
                 ^ literal(r3)
  10 + 10 + 10 + 10
               ^ add r2, r2, r3
  10 + 10 + 10 + 10
          ^ add r1, r1, r2
  10 + 10 + 10 + 10
     ^ add r0, r0, r1

  # A: No, we have to use extra registers where possible


*/
