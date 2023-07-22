#![allow(clippy::needless_lifetimes)]

mod builder;

use alloc::format;
use core::cmp::max;
use core::fmt::{Debug, Display};
use core::hash::BuildHasherDefault;

use beef::lean::Cow;
use bumpalo::collections::Vec;
use bumpalo::{vec, AllocErr};
use rustc_hash::FxHasher;

use self::builder::{BytecodeBuilder, ConstantPoolBuilder};
use super::{Mvar, Op, Reg, Upvalue};
use crate::ast::{
  Binary, BinaryOp, Block, Expr, Func, GetVar, If, Let, Lit, Logical, Loop, Module, Return, SetVar,
  Stmt, Unary, UnaryOp,
};
use crate::error::StdError;
use crate::gc::{Gc, Ref};
use crate::lex::Span;
use crate::obj::func::{Code, FunctionDescriptor, Params};
use crate::obj::list::ListDescriptor;
use crate::obj::module::ModuleDescriptor;
use crate::obj::string::Str;
use crate::obj::table::TableDescriptor;
use crate::obj::tuple::TupleDescriptor;
use crate::op::asm::*;
use crate::op::Smi;
use crate::{alloc, Arena};

pub type Result<T> = core::result::Result<T, EmitError>;

#[derive(Debug)]
pub struct EmitError {
  pub message: Cow<'static, str>,
}

impl EmitError {
  pub fn new(message: impl Into<Cow<'static, str>>) -> EmitError {
    EmitError {
      message: message.into(),
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

// type HashSet<T, A> = hashbrown::HashSet<T, BuildHasherDefault<FxHasher>, A>;
type HashMap<K, V, A> = hashbrown::HashMap<K, V, BuildHasherDefault<FxHasher>, A>;

struct ModuleState<'arena, 'gc, 'src> {
  arena: &'arena Arena,
  gc: &'gc Gc,
  name: &'src str,
  ast: Module<'arena, 'src>,

  src: Ref<Str>,
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

  params: Params,
  scope: Scope,
  locals: Vec<'arena, (Scope, &'src str, Reg<u8>)>,
}

#[derive(Clone, Copy, Default)]
struct Registers {
  current: u8,
  total: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Scope(usize);

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
  fn pool(&mut self) -> &mut ConstantPoolBuilder<'arena> {
    self.builder.pool()
  }

  #[inline]
  fn is_global_scope(&self) -> bool {
    self.is_top_level() && self.scope.0 <= 1
  }

  #[inline]
  fn is_top_level(&self) -> bool {
    self.parent.is_none()
  }

  /// Invariant: `reg` must already contain the value
  ///
  /// Note: This frees `reg` if necessary
  fn declare_var(&mut self, name: &'src str, reg: Reg<u8>, span: impl Into<Span>) -> Result<()> {
    if self.is_global_scope() {
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
      // `0` is `self` or the callee
      // `1..=params.max` are the params, e.g. `params.max=5`,
      // then `1, 2, 3, 4, 5` would be params
      if reg.wide() == 0 {
        Var::Self_
      } else if reg.wide() <= self.params.max as usize {
        Var::Param(reg)
      } else {
        Var::Local(reg)
      }
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
  let src = Str::try_new_in(gc, ast.src)?;
  let mut module = ModuleState {
    arena,
    gc,
    name,
    ast,

    src,
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

    params: Params::empty(),
    scope: Scope(0),
    locals: Vec::new_in(arena),
  };

  f.scope(|f| {
    for node in f.module.ast.body {
      stmt(f, node)?;
    }
    Ok(())
  })?;

  f.emit(finalize_module(), Span::empty())?;
  f.free(Reg(0));
  let dst = f.reg()?;
  f.emit(load_nil(dst), Span::empty())?;
  f.emit(ret(dst), Span::empty())?;

  let (ops, pool, spans, labels) = f.builder.finish();
  let code = Code {
    src: f.module.src,
    ops: &ops,
    pool: &pool,
    spans: &spans,
    locals: &f.locals[..],
    labels: &labels,
    stack_space: f.registers.total,
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
    assign_to(f, dst, value)?;
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

  let span = node.span;
  match node.kind {
    Logical(node) => logical(f, dst, node, span),
    Binary(node) => binary(f, dst, node, span),
    Unary(node) => unary(f, dst, node, span),
    Block(node) => block(f, dst, node),
    If(node) => if_(f, dst, node),
    Func(_) => todo!(),
    GetVar(node) => get_var(f, dst, node, span),
    SetVar(node) => set_var(f, node, span),
    GetField(_) => todo!(),
    SetField(_) => todo!(),
    GetIndex(_) => todo!(),
    SetIndex(_) => todo!(),
    Call(_) => todo!(),
    Lit(inner) => lit(f, dst, inner, span),
  }
}

fn logical<'arena, 'gc, 'src, 'state>(
  f: &mut FunctionState<'arena, 'gc, 'src, 'state>,
  dst: Option<Reg<u8>>,
  node: &Logical<'arena, 'src>,
  span: Span,
) -> Result<Option<Reg<u8>>> {
  todo!()
}

fn binary<'arena, 'gc, 'src, 'state>(
  f: &mut FunctionState<'arena, 'gc, 'src, 'state>,
  dst: Option<Reg<u8>>,
  node: &Binary<'arena, 'src>,
  span: Span,
) -> Result<Option<Reg<u8>>> {
  let lhs = expr(f, dst, &node.lhs)?.or(dst);
  let rhs = f.reg()?;
  let rhs = expr(f, Some(rhs), &node.rhs)?.unwrap_or(rhs);

  use BinaryOp::*;
  if let (Some(dst), Some(lhs)) = (dst, lhs) {
    match node.op {
      Add => f.emit(add(dst, lhs, rhs), span)?,
      Sub => f.emit(sub(dst, lhs, rhs), span)?,
      Div => f.emit(div(dst, lhs, rhs), span)?,
      Mul => f.emit(mul(dst, lhs, rhs), span)?,
      Rem => f.emit(rem(dst, lhs, rhs), span)?,
      Pow => f.emit(pow(dst, lhs, rhs), span)?,
      Eq => f.emit(cmp_eq(dst, lhs, rhs), span)?,
      Ne => f.emit(cmp_ne(dst, lhs, rhs), span)?,
      Gt => f.emit(cmp_gt(dst, lhs, rhs), span)?,
      Ge => f.emit(cmp_ge(dst, lhs, rhs), span)?,
      Lt => f.emit(cmp_lt(dst, lhs, rhs), span)?,
      Le => f.emit(cmp_le(dst, lhs, rhs), span)?,
    }
  }

  f.free(rhs);

  Ok(None)
}

fn unary<'arena, 'gc, 'src, 'state>(
  f: &mut FunctionState<'arena, 'gc, 'src, 'state>,
  dst: Option<Reg<u8>>,
  node: &Unary<'arena, 'src>,
  span: Span,
) -> Result<Option<Reg<u8>>> {
  if let Some(dst) = expr(f, dst, &node.rhs)?.or(dst) {
    use UnaryOp::*;
    match node.op {
      Min => f.emit(inv(dst), span)?,
      Not => f.emit(not(dst), span)?,
    }
  }
  Ok(None)
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

fn if_<'arena, 'gc, 'src, 'state>(
  f: &mut FunctionState<'arena, 'gc, 'src, 'state>,
  dst: Option<Reg<u8>>,
  node: &If<'arena, 'src>,
) -> Result<Option<Reg<u8>>> {
  todo!()
}

enum Var {
  Self_,
  Param(Reg<u8>),
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
    Self_ => Ok(Some(Reg(0))),
    Param(reg) => Ok(Some(reg)),
    Local(reg) => Ok(Some(reg)),
    Upvalue(idx) => {
      if let Some(dst) = dst {
        f.emit(load_upvalue(dst, idx), span)?;
      }
      Ok(None)
    }
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

fn set_var<'arena, 'gc, 'src, 'state>(
  f: &mut FunctionState<'arena, 'gc, 'src, 'state>,
  node: &SetVar<'arena, 'src>,
  span: Span,
) -> Result<Option<Reg<u8>>> {
  use Var::*;

  // TODO: test all cases here

  match f.resolve_var(node.name.lexeme) {
    Self_ => {
      return Err(EmitError::new(if f.params.has_self {
        Cow::borrowed("cannot assign to `self`")
      } else {
        Cow::owned(format!("cannot assign to function `{}`", node.name.lexeme))
      }));
    }
    Param(_) => {
      return Err(EmitError::new(format!(
        "cannot assign to parameter `{}`",
        node.name.lexeme
      )));
    }
    Local(reg) => {
      assign_to(f, reg, &node.value)?;
    }
    Upvalue(_) => {
      return Err(EmitError::new(format!(
        "cannot assign to non-local variable `{}`",
        node.name.lexeme
      )))
    }
    Module(idx) => {
      let dst = f.reg()?;
      let out = expr(f, Some(dst), &node.value)?.unwrap_or(dst);
      f.emit(set_mvar(out, idx), span)?;
      f.free(dst);
    }
    Global => {
      let name = Str::try_intern_in(f.gc, node.name.lexeme)?;
      let name = f.pool().str(name)?;
      let dst = f.reg()?;
      let out = expr(f, Some(dst), &node.value)?.unwrap_or(dst);
      f.emit(set_global(out, name), span)?;
      f.free(dst);
    }
  }

  Ok(None)
}

fn assign_to<'arena, 'gc, 'src, 'state>(
  f: &mut FunctionState<'arena, 'gc, 'src, 'state>,
  dst: Reg<u8>,
  value: &Expr<'arena, 'src>,
) -> Result<()> {
  if let Some(out) = expr(f, Some(dst), value)? {
    // `expr` was written to `out`
    f.emit(mov(out, dst), value.span)?;
  } else {
    // `expr` was written to `dst`
  }

  Ok(())
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
    Float(v) => {
      let v = f.pool().float(*v)?;
      f.emit(load_const(dst, v), span)?;
    }
    Int(value) => {
      if let Ok(value) = i16::try_from(*value) {
        f.emit(load_smi(dst, Smi(value)), span)?;
      } else {
        // constant + emit load const
        todo!()
      }
    }
    Nil => {
      f.emit(load_nil(dst), span)?;
    }
    Bool(v) => match *v {
      true => f.emit(load_true(dst), span)?,
      false => f.emit(load_false(dst), span)?,
    },
    String(v) => {
      let v = Str::try_intern_in(f.gc, v)?;
      let v = f.pool().str(v)?;
      f.emit(load_const(dst, v), span)?;
    }
    Record([]) => {
      f.emit(make_table_empty(dst), span)?;
    }
    Record(fields) => {
      let mut regs = vec![in f.arena];
      let mut keys = vec![in f.arena];
      for _ in fields.iter() {
        regs.push(f.reg()?);
      }
      for ((key, value), reg) in fields.iter().zip(regs.iter()) {
        assign_to(f, *reg, value)?;
        keys.push(Str::try_intern_in(f.gc, key.lexeme)?);
      }
      let desc = TableDescriptor::try_new_in(f.gc, regs[0], &keys)?;
      let desc = f.pool().table(desc)?;
      f.emit(make_table(dst, desc), span)?;
      f.free(regs[0]);
    }
    List([]) => {
      f.emit(make_list_empty(dst), span)?;
    }
    List(items) => {
      let mut regs = vec![in f.arena];
      for _ in items.iter() {
        regs.push(f.reg()?);
      }
      for (value, reg) in items.iter().zip(regs.iter()) {
        assign_to(f, *reg, value)?;
      }
      let desc = ListDescriptor::try_new_in(f.gc, regs[0], regs.len() as u8)?;
      let desc = f.pool().list(desc)?;
      f.emit(make_list(dst, desc), span)?;
      f.free(regs[0]);
    }
    Tuple([]) => {
      f.emit(make_tuple_empty(dst), span)?;
    }
    Tuple(elems) => {
      let mut regs = vec![in f.arena];
      for _ in elems.iter() {
        regs.push(f.reg()?);
      }
      for (value, reg) in elems.iter().zip(regs.iter()) {
        assign_to(f, *reg, value)?;
      }
      let desc = TupleDescriptor::try_new_in(f.gc, regs[0], regs.len() as u8)?;
      let desc = f.pool().tuple(desc)?;
      f.emit(make_tuple(dst, desc), span)?;
      f.free(regs[0]);
    }
  }

  Ok(None)
}
