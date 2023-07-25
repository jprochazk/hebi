#![allow(clippy::needless_lifetimes)]

pub mod builder;

use alloc::format;
use core::cmp::max;
use core::fmt::{Debug, Display};
use core::mem::replace;

use beef::lean::Cow;
use bumpalo::collections::Vec;
use bumpalo::vec;

use self::builder::{BytecodeBuilder, ConstantPoolBuilder, LoopLabel, MultiLabel};
use super::{Capture, Const, Mvar, Op, Reg};
use crate::ast::{
  Binary, BinaryOp, Block, Expr, Func, GetField, GetIndex, GetVar, Ident, If, Key, Let, Lit,
  Logical, LogicalOp, Loop, Module, Name, Return, SetField, SetIndex, SetVar, Stmt, Unary, UnaryOp,
};
use crate::ds::fx;
use crate::ds::map::BumpHashMap;
use crate::error::{AllocError, StdError};
use crate::gc::{Gc, Ref};
use crate::lex::Span;
use crate::obj::func::{Code, FunctionDescriptor, Params};
use crate::obj::list::ListDescriptor;
use crate::obj::module::ModuleDescriptor;
use crate::obj::string::Str;
use crate::obj::table::TableDescriptor;
use crate::obj::tuple::TupleDescriptor;
use crate::op::asm::*;
use crate::op::emit::builder::BasicLabel;
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

impl From<AllocError> for EmitError {
  fn from(e: AllocError) -> Self {
    Self::new(format!("{}", e))
  }
}

impl Display for EmitError {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "error: {}", self.message)
  }
}

impl StdError for EmitError {}

// TODO: investigate why `emit__assign_field` uses 5 registers
// the disassembly only shows `r0, r1, r2, r3`. where's `r4`?

struct Compiler<'arena, 'gc, 'src> {
  arena: &'arena Arena,
  gc: &'gc Gc,
  ast: Module<'arena, 'src>,

  src: Ref<Str>,
  /// This is a map of top-level variables, a.k.a. global variables.
  /// In hebi they're referred to as "module" variables, because
  /// they're instantiated per module.
  vars: BumpHashMap<'arena, Cow<'src, str>, Mvar<u16>>,

  funcs: Vec<'arena, Function<'arena, 'src>>,
}

struct Function<'arena, 'src> {
  loop_: Option<LoopState<'arena>>,

  name: Cow<'src, str>,

  builder: BytecodeBuilder<'arena>,
  registers: Registers,

  params: Params,
  scopes: Vec<'arena, Scope<'arena, 'src>>,
  captures: Vec<'arena, (Cow<'src, str>, CaptureInfo)>,
}

impl<'arena, 'src> Function<'arena, 'src> {
  fn resolve_local(&self, name: &'src str) -> Option<Reg<u8>> {
    for scope in self.scopes.iter().rev() {
      for local in scope.locals.iter().rev() {
        if local.name == name {
          return Some(local.reg);
        }
      }
    }
    None
  }

  fn resolve_local_in_scope(&self, name: &'src str) -> Option<Reg<u8>> {
    let scope = self.scopes.last().unwrap();
    for local in scope.locals.iter().rev() {
      if local.name == name {
        return Some(local.reg);
      }
    }
    None
  }
}

pub struct Scope<'arena, 'src> {
  pub locals: Vec<'arena, LocalVar<'src>>,
}

impl<'arena, 'src> Scope<'arena, 'src> {
  fn new_in(arena: &'arena Arena) -> Self {
    Self {
      locals: Vec::new_in(arena),
    }
  }
}

#[derive(Clone, Copy)]
pub enum CaptureInfo {
  NonLocal {
    src: Reg<u8>,
    dst: Capture<u16>,
  },
  Parent {
    src: Capture<u16>,
    dst: Capture<u16>,
  },
}

impl CaptureInfo {
  pub fn dst(&self) -> Capture<u16> {
    match self {
      CaptureInfo::NonLocal { dst, .. } => *dst,
      CaptureInfo::Parent { dst, .. } => *dst,
    }
  }
}

pub struct LocalVar<'src> {
  pub name: Cow<'src, str>,
  pub reg: Reg<u8>,
}

struct LoopStateBasic<'arena> {
  continue_to: LoopLabel,
  break_to: MultiLabel<'arena>,
}
struct LoopStateLatched<'arena> {
  continue_to: MultiLabel<'arena>,
  break_to: MultiLabel<'arena>,
}
enum LoopState<'arena> {
  Basic(LoopStateBasic<'arena>),
  Latched(LoopStateLatched<'arena>),
}

#[allow(dead_code, clippy::wrong_self_convention)]
impl<'arena> LoopState<'arena> {
  fn basic(continue_to: LoopLabel, break_to: MultiLabel<'arena>) -> Self {
    Self::Basic(LoopStateBasic {
      continue_to,
      break_to,
    })
  }

  fn latched(continue_to: MultiLabel<'arena>, break_to: MultiLabel<'arena>) -> Self {
    Self::Latched(LoopStateLatched {
      continue_to,
      break_to,
    })
  }

  fn to_basic(self) -> LoopStateBasic<'arena> {
    use LoopState::*;
    match self {
      Basic(state) => state,
      _ => panic!("expected LoopStateBasic"),
    }
  }

  fn to_latched(self) -> LoopStateLatched<'arena> {
    use LoopState::*;
    match self {
      Latched(state) => state,
      _ => panic!("expected LoopStateLatched"),
    }
  }
}

#[derive(Clone, Copy, Default)]
struct Registers {
  current: u8,
  total: u8,
}

macro_rules! fmut {
  ($self:expr) => {{
    let _f = ($self).funcs.last_mut();
    debug_assert!(_f.is_some());
    unsafe { _f.unwrap_unchecked() }
  }};
}

macro_rules! fs {
  ($self:expr) => {{
    let _f = ($self).funcs.last();
    debug_assert!(_f.is_some());
    unsafe { _f.unwrap_unchecked() }
  }};
}

impl<'arena, 'gc, 'src> Compiler<'arena, 'gc, 'src> {
  #[inline]
  fn scope<F, T>(&mut self, f: F) -> Result<T>
  where
    F: FnOnce(&mut Compiler<'arena, 'gc, 'src>) -> Result<T>,
  {
    let base = Reg(fs!(self).registers.current);
    fmut!(self).scopes.push(Scope::new_in(self.arena));
    let result = f(self);
    fmut!(self).scopes.pop();
    self.free(base);
    result
  }

  #[doc(hidden)]
  #[inline]
  fn _reg(&mut self) -> Reg<u8> {
    let func = fmut!(self);
    let reg = func.registers.current;
    func.registers.current += 1;
    func.registers.total = max(func.registers.current, func.registers.total);
    Reg(reg)
  }

  #[inline]
  fn reg(&mut self) -> Result<Reg<u8>> {
    let func = fmut!(self);
    if func.registers.current == u8::MAX {
      return Err(EmitError::new(format!(
        "function `{}` uses too many registers, maximum is {}",
        func.name,
        u8::MAX
      )));
    }
    Ok(self._reg())
  }

  #[inline]
  fn free(&mut self, r: Reg<u8>) {
    fmut!(self).registers.current = r.0;
  }

  #[inline]
  fn emit(&mut self, op: Op, span: impl Into<Span>) -> Result<()> {
    self.builder().emit(op, span)?;
    Ok(())
  }

  #[inline]
  fn pool(&mut self) -> &mut ConstantPoolBuilder<'arena> {
    self.builder().pool()
  }

  #[inline]
  fn builder(&mut self) -> &mut BytecodeBuilder<'arena> {
    &mut fmut!(self).builder
  }

  #[inline]
  fn is_global_scope(&self) -> bool {
    self.is_top_level() && fs!(self).scopes.len() <= 1
  }

  #[inline]
  fn is_top_level(&self) -> bool {
    self.funcs.len() == 1
  }

  /// Invariant: `reg` must already contain the value
  ///
  /// Note: This frees `reg` if necessary
  fn declare_var(
    &mut self,
    name: Cow<'src, str>,
    reg: Reg<u8>,
    span: impl Into<Span>,
  ) -> Result<()> {
    if self.is_global_scope() {
      // module variable
      // value is in `reg`, we have to add the var to module.vars
      let last = self.vars.len();
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
      let idx = *self.vars.entry(name).or_insert_with(|| Mvar(last));
      self.emit(store_mvar(reg, idx), span)?;
      self.free(reg);
    } else {
      // local variable
      // no need to emit anything, just add it to locals
      let func = fmut!(self);
      if !func
        .scopes
        .last()
        .unwrap()
        .locals
        .iter()
        .any(|v| v.name == name)
      {
        func
          .scopes
          .last_mut()
          .unwrap()
          .locals
          .push(LocalVar { name, reg });
      }
      // note: doing nothing is fine if `locals` already contains
      // `(scope, name)`, `reg` is already reusing an existing register
      // if possible, and it's already set to the correct value.
    }

    Ok(())
  }

  fn resolve_var(&mut self, name: &'src str) -> Result<Var> {
    if self.is_top_level() {
      if let Some(reg) = fs!(self).resolve_local(name) {
        Ok(Var::Local(reg))
      } else if let Some(idx) = self.vars.get(name).copied() {
        Ok(Var::Module(idx))
      } else {
        Ok(Var::Global)
      }
    } else if let Some(reg) = fs!(self).resolve_local(name) {
      // `0` is `self` or the callee
      // `1..=params.max` are the params, e.g. `params.max=5`,
      // then `1, 2, 3, 4, 5` would be params
      if reg.wide() == 0 {
        Ok(Var::Self_)
      } else if reg.wide() <= fs!(self).params.max as usize {
        Ok(Var::Param(reg))
      } else {
        Ok(Var::Local(reg))
      }
    } else if let Some(idx) = self.resolve_capture(name)? {
      Ok(Var::Capture(idx))
    } else if let Some(idx) = self.vars.get(name).copied() {
      Ok(Var::Module(idx))
    } else {
      Ok(Var::Global)
    }
  }

  fn resolve_capture(&mut self, name: &'src str) -> Result<Option<Capture<u16>>> {
    fn inner<'arena, 'gc, 'src>(
      c: &mut Compiler<'arena, 'gc, 'src>,
      name: &'src str,
      idx: usize,
    ) -> Result<Option<Capture<u16>>> {
      if idx == 0 {
        return Ok(None);
      }

      macro_rules! f {
        ($c:ident, $i:expr) => {
          $c.funcs[$i]
        };
      }

      macro_rules! u {
        ($c:ident, $i:expr) => {
          $c.funcs[$i].captures
        };
      }

      if let Some((_, uv)) = u!(c, idx).iter().find(|(n, _)| n == name) {
        return Ok(Some(uv.dst()));
      }

      if u!(c, idx).len() > u16::MAX as usize {
        return Err(EmitError::new(format!(
          "too many captures in function `{}`",
          f!(c, idx).name
        )));
      }

      let dst = Capture(u!(c, idx).len() as u16);
      if let Some(src) = f!(c, idx - 1).resolve_local(name) {
        u!(c, idx).push((name.into(), CaptureInfo::NonLocal { src, dst }));
        return Ok(Some(dst));
      }

      if let Some(src) = inner(c, name, idx - 1)? {
        u!(c, idx).push((name.into(), CaptureInfo::Parent { src, dst }));
        return Ok(Some(src));
      }

      Ok(None)
    }

    let top = self.funcs.len().saturating_sub(1);
    inner(self, name, top)
  }
}

pub fn module<'arena, 'gc, 'src>(
  arena: &'arena Arena,
  gc: &'gc Gc,
  name: &'src str,
  ast: Module<'arena, 'src>,
) -> Result<Ref<ModuleDescriptor>> {
  let src = Str::try_new_in(gc, ast.src)?;
  let mut module = Compiler {
    arena,
    gc,
    ast,

    src,
    vars: BumpHashMap::with_hasher_in(fx(), arena),
    funcs: Vec::new_in(arena),
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
  c: &mut Compiler<'arena, 'gc, 'src>,
  arena: &'arena Arena,
  gc: &'gc Gc,
) -> Result<Ref<FunctionDescriptor>> {
  c.funcs.push(Function {
    loop_: None,

    name: "__main__".into(),

    builder: BytecodeBuilder::new_in(arena),
    registers: Registers::default(),

    params: Params::empty(),
    scopes: Vec::new_in(arena),
    captures: Vec::new_in(arena),
  });

  c.scope(|c| {
    for node in c.ast.body {
      stmt(c, node)?;
    }
    Ok(())
  })?;

  c.emit(finalize_module(), Span::empty())?;
  c.free(Reg(0));
  let dst = c.reg()?;
  c.emit(load_nil(dst), Span::empty())?;
  c.emit(ret(dst), Span::empty())?;

  let func = c.funcs.pop().unwrap();

  let (ops, pool, spans, label_map) = func.builder.finish();
  let code = Code {
    src: c.src,
    ops: &ops,
    pool: &pool,
    spans: &spans,
    label_map,
    stack_space: func.registers.total,
    captures: &func.captures,
  };

  Ok(FunctionDescriptor::try_new_in(
    gc,
    &func.name,
    Params::empty(),
    code,
  )?)
}

fn stmt<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  node: &Stmt<'arena, 'src>,
) -> Result<()> {
  use crate::ast::StmtKind::*;

  let span = node.span;
  match node.kind {
    Let(inner) => let_(c, inner, span),
    Loop(inner) => loop_(c, inner),
    Break => break_(c, span),
    Continue => continue_(c, span),
    Return(inner) => return_(c, inner, span),
    Func(inner) => func_stmt(c, inner),
    Expr(inner) => {
      let _ = expr(c, None, inner)?;
      Ok(())
    }
  }
}

fn func_stmt<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  node: &Func<'arena, 'src>,
) -> Result<()> {
  let name = Name::from(node.name.unwrap());
  let func = function(c, node.params, &name, &node.body, false)?;
  let dst = match fs!(c).resolve_local_in_scope(func.name().as_str()) {
    Some(reg) => reg,
    None => c.reg()?,
  };
  let func = c.pool().func(func)?;
  c.emit(load_const(dst, func), name.span)?;
  c.declare_var(name.lexeme, dst, name.span)
}

fn let_<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  node: &Let<'arena, 'src>,
  span: Span,
) -> Result<()> {
  // note: `declare_var` at the end is responsible for freeing `dst` if necessary
  let dst = match fs!(c).resolve_local_in_scope(node.name.lexeme) {
    Some(reg) => reg,
    None => c.reg()?,
  };

  if let Some(value) = &node.value {
    assign_to(c, dst, value)?;
  } else {
    c.emit(load_nil(dst), span)?;
  }

  c.declare_var(node.name.lexeme.into(), dst, span)?;

  Ok(())
}

fn loop_body<'arena, 'gc, 'src, F>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  loop_: LoopState<'arena>,
  f: F,
) -> Result<LoopState<'arena>>
where
  F: FnOnce(&mut Compiler<'arena, 'gc, 'src>) -> Result<()>,
{
  let prev = fmut!(c).loop_.replace(loop_);
  let result = f(c);
  let state = replace(&mut fmut!(c).loop_, prev).unwrap();
  result?;
  Ok(state)
}

macro_rules! build_loop {
  ($name:literal, ($c:ident)
    start:$start:block
    body:$body:block
  ) => {{
    #![allow(unused_variables, clippy::redundant_closure_call)]

    let mut lstart = LoopLabel::new($c.builder(), concat!($name, "::start"));
    let lend = MultiLabel::new($c.builder(), concat!($name, "::end"));

    lstart.bind($c.builder());
    (|$c| Ok::<(), EmitError>($start))(&mut *$c)?;

    let LoopStateBasic {
      continue_to: lstart,
      break_to: lend,
    } = loop_body($c, LoopState::basic(lstart, lend), (|$c| Ok($body)))?.to_basic();

    lstart.emit($c.builder(), jump_loop, Span::empty())?;
    lend.bind($c.builder())
  }};

  (latched $name:literal, ($c:ident)
    start:$start:block
    body:$body:block
    latch:$latch:block
  ) => {{
    #![allow(unused_variables, clippy::redundant_closure_call)]

    let index = $c.builder().label_index();

    let mut lstart = LoopLabel::new($c.builder(), concat!($name, "::start"));
    let llatch = MultiLabel::new($c.builder(), concat!($name, "::latch"));
    let lend = MultiLabel::new($c.builder(), concat!($name, "::end"));

    lstart.bind($c.builder());
    (|$c| Ok::<(), EmitError>($start))(&mut *$c)?;

    let LoopStateLatched {
      continue_to: llatch,
      break_to: lend,
    } = loop_body($c, LoopState::latched(llatch, lend), (|$c| Ok($body)))?.to_latched();

    llatch.bind($c.builder())?;
    (|$c| Ok::<(), EmitError>($latch))(&mut *$c)?;

    lstart.emit($c.builder(), jump_loop, Span::empty())?;
    lend.bind($c.builder())
  }};
}

fn loop_<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  node: &Loop<'arena, 'src>,
) -> Result<()> {
  build_loop!("loop", (c)
    start: {}
    body: {
      block(c, None, &node.body)?;
    }
  )
}

fn break_<'arena, 'gc, 'src>(c: &mut Compiler<'arena, 'gc, 'src>, span: Span) -> Result<()> {
  let f = fmut!(c);
  let Some(loop_) = f.loop_.as_mut() else {
    return Err(EmitError::new("cannot use `break` outside of loop"));
  };
  match loop_ {
    LoopState::Basic(state) => state.break_to.emit(&mut f.builder, jump, span),
    LoopState::Latched(state) => state.break_to.emit(&mut f.builder, jump, span),
  }
}

fn continue_<'arena, 'gc, 'src>(c: &mut Compiler<'arena, 'gc, 'src>, span: Span) -> Result<()> {
  let f = fmut!(c);
  let Some(loop_) = f.loop_.as_mut() else {
    return Err(EmitError::new("cannot use `continue` outside of loop"));
  };
  match loop_ {
    LoopState::Basic(state) => state.continue_to.emit(&mut f.builder, jump_loop, span),
    LoopState::Latched(state) => state.continue_to.emit(&mut f.builder, jump, span),
  }
}

fn return_<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  node: &Return<'arena, 'src>,
  span: Span,
) -> Result<()> {
  if c.is_top_level() {
    return Err(EmitError::new("cannot use `return` outside of function"));
  }

  let tmp = c.reg()?;
  match &node.value {
    Some(value) => {
      assign_to(c, tmp, value)?;
    }
    None => {
      c.emit(load_nil(tmp), span)?;
    }
  }
  c.emit(ret(tmp), span)?;
  c.free(tmp);

  Ok(())
}

fn function<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  params: &[Ident<'src>],
  name: &Name<'src>,
  block: &Block<'arena, 'src>,
  is_anon: bool,
) -> Result<Ref<FunctionDescriptor>> {
  c.funcs.push(Function {
    loop_: None,

    name: name.lexeme.clone(),

    builder: BytecodeBuilder::new_in(c.arena),
    registers: Registers::default(),

    params: Params {
      min: params.len() as u8,
      max: params.len() as u8,
      has_self: false,
    },
    scopes: Vec::new_in(c.arena),
    captures: Vec::new_in(c.arena),
  });

  c.scope(|c| {
    let fn_reg = c.reg()?;
    if !is_anon {
      c.declare_var(name.lexeme.clone(), fn_reg, name.span)?;
    }
    for param in params {
      let reg = c.reg()?;
      c.declare_var(param.lexeme.into(), reg, param.span)?;
    }

    for node in block.body {
      stmt(c, node)?;
    }

    let out = c.reg()?;
    if let Some(last) = &block.last {
      assign_to(c, out, last)?;
      c.emit(ret(out), Span::empty())?;
    }

    Ok(())
  })?;

  c.free(Reg(0));
  let dst = c.reg()?;
  c.emit(load_nil(dst), Span::empty())?;
  c.emit(ret(dst), Span::empty())?;

  let func = c.funcs.pop().unwrap();

  let (ops, pool, spans, label_map) = func.builder.finish();
  let code = Code {
    src: c.src,
    ops: &ops,
    pool: &pool,
    spans: &spans,
    label_map,
    stack_space: func.registers.total,
    captures: &func.captures,
  };

  Ok(FunctionDescriptor::try_new_in(
    c.gc,
    &func.name,
    Params::empty(),
    code,
  )?)
}

fn expr<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  dst: Option<Reg<u8>>,
  node: &Expr<'arena, 'src>,
) -> Result<Option<Reg<u8>>> {
  use crate::ast::ExprKind::*;

  let span = node.span;
  match node.kind {
    Logical(node) => logical(c, dst, node),
    Binary(node) => binary(c, dst, node, span),
    Unary(node) => unary(c, dst, node, span),
    Block(node) => block(c, dst, node).map(|_| None),
    If(node) => if_(c, dst, node),
    Func(node) => func_expr(c, dst, node).map(|_| None),
    GetVar(node) => get_var(c, dst, node, span),
    SetVar(node) => set_var(c, node, span),
    GetField(node) => get_field(c, dst, node, span),
    SetField(node) => set_field(c, node, span),
    GetIndex(node) => get_index(c, dst, node, span),
    SetIndex(node) => set_index(c, node, span),
    Call(_) => todo!(),
    Lit(inner) => lit(c, dst, inner, span),
  }
}

fn func_expr<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  dst: Option<Reg<u8>>,
  f: &Func<'arena, 'src>,
) -> Result<()> {
  if let Some(dst) = dst {
    let name = match f.name {
      Some(name) => name.into(),
      None => Name::fake(f.fn_token_span, format!("[fn@{}]", f.fn_token_span)),
    };
    let func = function(c, f.params, &name, &f.body, f.name.is_none())?;
    let func = c.pool().func(func)?;
    c.emit(load_const(dst, func), name.span)?;
  }

  Ok(())
}

fn logical<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  dst: Option<Reg<u8>>,
  node: &Logical<'arena, 'src>,
) -> Result<Option<Reg<u8>>> {
  use LogicalOp::*;

  let (free, dst) = match dst {
    Some(dst) => (false, dst),
    None => (true, c.reg()?),
  };
  let name = match node.op {
    And => "and",
    Or => "or",
  };
  let mut use_lhs = BasicLabel::new(c.builder(), name);

  assign_to(c, dst, &node.lhs)?;
  match node.op {
    And => {
      use_lhs.emit(c.builder(), jump_if_false(dst), node.lhs.span)?;
    }
    Or => {
      use_lhs.emit(c.builder(), jump_if_true(dst), node.lhs.span)?;
    }
  }
  assign_to(c, dst, &node.rhs)?;
  use_lhs.bind(c.builder())?;

  if free {
    c.free(dst);
  }

  Ok(None)
}

fn binary<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  dst: Option<Reg<u8>>,
  node: &Binary<'arena, 'src>,
  span: Span,
) -> Result<Option<Reg<u8>>> {
  let (free, dst) = match dst {
    Some(dst) => (false, dst),
    None => (true, c.reg()?),
  };

  let lhs = expr(c, Some(dst), &node.lhs)?.unwrap_or(dst);
  let rhs = c.reg()?;
  let rhs = expr(c, Some(rhs), &node.rhs)?.unwrap_or(rhs);

  use BinaryOp::*;
  match node.op {
    Add => c.emit(add(dst, lhs, rhs), span)?,
    Sub => c.emit(sub(dst, lhs, rhs), span)?,
    Div => c.emit(div(dst, lhs, rhs), span)?,
    Mul => c.emit(mul(dst, lhs, rhs), span)?,
    Rem => c.emit(rem(dst, lhs, rhs), span)?,
    Pow => c.emit(pow(dst, lhs, rhs), span)?,
    Eq => c.emit(cmp_eq(dst, lhs, rhs), span)?,
    Ne => c.emit(cmp_ne(dst, lhs, rhs), span)?,
    Gt => c.emit(cmp_gt(dst, lhs, rhs), span)?,
    Ge => c.emit(cmp_ge(dst, lhs, rhs), span)?,
    Lt => c.emit(cmp_lt(dst, lhs, rhs), span)?,
    Le => c.emit(cmp_le(dst, lhs, rhs), span)?,
  }

  c.free(rhs);
  if free {
    c.free(dst)
  }

  Ok(None)
}

fn unary<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  dst: Option<Reg<u8>>,
  node: &Unary<'arena, 'src>,
  span: Span,
) -> Result<Option<Reg<u8>>> {
  if let Some(dst) = expr(c, dst, &node.rhs)?.or(dst) {
    use UnaryOp::*;
    match node.op {
      Min => c.emit(inv(dst), span)?,
      Not => c.emit(not(dst), span)?,
    }
  }
  Ok(None)
}

fn block<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  dst: Option<Reg<u8>>,
  node: &Block<'arena, 'src>,
) -> Result<()> {
  c.scope(|c| {
    for node in node.body {
      stmt(c, node)?;
    }

    if let Some(last) = &node.last {
      match dst {
        Some(dst) => assign_to(c, dst, last)?,
        None => expr(c, dst, last).map(|_| ())?,
      };
    }

    Ok(())
  })
}

fn if_<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  dst: Option<Reg<u8>>,
  node: &If<'arena, 'src>,
) -> Result<Option<Reg<u8>>> {
  let (free, dst) = match dst {
    Some(dst) => (false, dst),
    None => (true, c.reg()?),
  };

  let mut end = MultiLabel::new(c.builder(), "if::end");

  let mut branches = node.br.iter().peekable();
  while let Some(branch) = branches.next() {
    let mut next = BasicLabel::new(c.builder(), "if::next");
    assign_to(c, dst, &branch.cond)?;
    next.emit(c.builder(), jump_if_false(dst), branch.cond.span)?;
    block(c, Some(dst), &branch.body)?;
    if node.tail.is_some() || branches.peek().is_some() {
      end.emit(c.builder(), jump, Span::empty())?;
    }
    next.bind(c.builder())?;
  }

  if let Some(tail) = &node.tail {
    block(c, Some(dst), tail)?;
  }

  end.bind(c.builder())?;

  if free {
    c.free(dst);
  }

  Ok(None)
}

enum Var {
  Self_,
  Param(Reg<u8>),
  Local(Reg<u8>),
  Capture(Capture<u16>),
  Module(Mvar<u16>),
  Global,
}

fn get_var<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  dst: Option<Reg<u8>>,
  node: &GetVar<'src>,
  span: Span,
) -> Result<Option<Reg<u8>>> {
  use Var::*;

  match c.resolve_var(node.name.lexeme)? {
    Self_ => Ok(Some(Reg(0))),
    Param(reg) => Ok(Some(reg)),
    Local(reg) => Ok(Some(reg)),
    Capture(idx) => {
      if let Some(dst) = dst {
        c.emit(load_capture(dst, idx), span)?;
      }
      Ok(None)
    }
    Module(var) => {
      if let Some(dst) = dst {
        c.emit(load_mvar(dst, var), span)?;
      }
      Ok(None)
    }
    Global => {
      if let Some(dst) = dst {
        let name = Str::try_intern_in(c.gc, node.name.lexeme)?;
        let name = c.pool().str(name)?;
        c.emit(load_global(dst, name), span)?;
      }
      Ok(None)
    }
  }
}

fn set_var<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  node: &SetVar<'arena, 'src>,
  span: Span,
) -> Result<Option<Reg<u8>>> {
  use Var::*;

  // TODO: test all cases here
  match c.resolve_var(node.name.lexeme)? {
    Self_ => {
      let msg = if fmut!(c).params.has_self {
        Cow::borrowed("cannot assign to `self`")
      } else {
        Cow::owned(format!("cannot assign to function `{}`", node.name.lexeme))
      };
      return Err(EmitError::new(msg));
    }
    Param(_) => {
      return Err(EmitError::new(format!(
        "cannot assign to parameter `{}`",
        node.name.lexeme
      )));
    }
    Local(reg) => {
      assign_to(c, reg, &node.value)?;
    }
    Capture(_) => {
      return Err(EmitError::new(format!(
        "cannot assign to non-local variable `{}`",
        node.name.lexeme
      )))
    }
    Module(idx) => {
      let dst = c.reg()?;
      let out = expr(c, Some(dst), &node.value)?.unwrap_or(dst);
      c.emit(store_mvar(out, idx), span)?;
      c.free(dst);
    }
    Global => {
      let name = Str::try_intern_in(c.gc, node.name.lexeme)?;
      let name = c.pool().str(name)?;
      let dst = c.reg()?;
      let out = expr(c, Some(dst), &node.value)?.unwrap_or(dst);
      c.emit(store_global(out, name), span)?;
      c.free(dst);
    }
  }

  Ok(None)
}

fn get_field<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  dst: Option<Reg<u8>>,
  node: &GetField<'arena, 'src>,
  span: Span,
) -> Result<Option<Reg<u8>>> {
  let (free, dst) = match dst {
    Some(dst) => (false, dst),
    None => (true, c.reg()?),
  };

  let obj_r = c.reg()?;
  let obj = expr(c, Some(obj_r), node.target)?.unwrap_or(obj_r);

  match field_key(c, node.key, span)? {
    FieldKey::IntConst(key) => {
      c.emit(load_field_int(obj, key, dst), span)?;
    }
    FieldKey::IntReg(key) => {
      c.emit(load_field_int_r(obj, key, dst), span)?;
      c.free(key);
    }
    FieldKey::StrConst(key) => {
      c.emit(load_field(obj, key, dst), span)?;
    }
    FieldKey::StrReg(key) => {
      c.emit(load_field_r(obj, key, dst), span)?;
      c.free(key);
    }
  }

  c.free(obj_r);
  if free {
    c.free(dst);
  }

  Ok(None)
}

enum FieldKey {
  IntConst(Const<u8>),
  IntReg(Reg<u8>),
  StrConst(Const<u8>),
  StrReg(Reg<u8>),
}

fn set_field<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  node: &SetField<'arena, 'src>,
  span: Span,
) -> Result<Option<Reg<u8>>> {
  let obj_r = c.reg()?;
  let val_r = c.reg()?;

  let obj = expr(c, Some(obj_r), node.target)?.unwrap_or(obj_r);
  let val = expr(c, Some(val_r), &node.value)?.unwrap_or(val_r);
  match field_key(c, node.key, span)? {
    FieldKey::IntConst(key) => {
      c.emit(store_field_int(obj, key, val), span)?;
    }
    FieldKey::IntReg(key) => {
      c.emit(store_field_int_r(obj, key, val), span)?;
      c.free(key);
    }
    FieldKey::StrConst(key) => {
      c.emit(store_field(obj, key, val), span)?;
    }
    FieldKey::StrReg(key) => {
      c.emit(store_field_r(obj, key, val), span)?;
      c.free(key);
    }
  }

  c.free(val_r);
  c.free(obj_r);

  Ok(None)
}

fn field_key(c: &mut Compiler, key: &Key, span: Span) -> Result<FieldKey> {
  use crate::ast::Key::*;
  match key {
    Int(key) => {
      let key = c.pool().int(*key)?;
      if key.is_u8() {
        Ok(FieldKey::IntConst(key.u8()))
      } else {
        let tmp1 = c.reg()?;
        c.emit(load_const(tmp1, key), span)?;
        Ok(FieldKey::IntReg(tmp1))
      }
    }
    Ident(key) => {
      let key = Str::try_intern_in(c.gc, key.lexeme)?;
      let key = c.pool().str(key)?;
      if key.is_u8() {
        Ok(FieldKey::StrConst(key.u8()))
      } else {
        let tmp1 = c.reg()?;
        c.emit(load_const(tmp1, key), span)?;
        Ok(FieldKey::StrReg(tmp1))
      }
    }
  }
}

// TODO: specialize index ops for constant keys
fn get_index<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  dst: Option<Reg<u8>>,
  node: &GetIndex<'arena, 'src>,
  span: Span,
) -> Result<Option<Reg<u8>>> {
  let (free, dst) = match dst {
    Some(dst) => (false, dst),
    None => (true, c.reg()?),
  };

  let obj_r = c.reg()?;
  let key_r = c.reg()?;

  let obj = expr(c, Some(obj_r), node.target)?.unwrap_or(obj_r);
  let key = expr(c, Some(key_r), node.index)?.unwrap_or(key_r);
  c.emit(load_index(obj, key, dst), span)?;

  c.free(key_r);
  c.free(obj_r);
  if free {
    c.free(dst);
  }

  Ok(None)
}

fn set_index<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  node: &SetIndex<'arena, 'src>,
  span: Span,
) -> Result<Option<Reg<u8>>> {
  let dst_r = c.reg()?;
  let key_r = c.reg()?;
  let val_r = c.reg()?;

  let obj = expr(c, Some(dst_r), node.target)?.unwrap_or(dst_r);
  let key = expr(c, Some(key_r), node.index)?.unwrap_or(key_r);
  let value = expr(c, Some(val_r), &node.value)?.unwrap_or(val_r);
  c.emit(store_index(obj, key, value), span)?;

  c.free(val_r);
  c.free(key_r);
  c.free(dst_r);

  Ok(None)
}

fn assign_to<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  dst: Reg<u8>,
  value: &Expr<'arena, 'src>,
) -> Result<()> {
  if let Some(out) = expr(c, Some(dst), value)? {
    // `expr` was written to `out`
    c.emit(mov(out, dst), value.span)?;
  } else {
    // `expr` was written to `dst`
  }

  Ok(())
}

fn lit<'arena, 'gc, 'src>(
  c: &mut Compiler<'arena, 'gc, 'src>,
  dst: Option<Reg<u8>>,
  node: &Lit<'arena, 'src>,
  span: Span,
) -> Result<Option<Reg<u8>>> {
  use Lit::*;

  let Some(dst) = dst else { return Ok(None) };

  match node {
    Float(v) => {
      let v = c.pool().float(*v)?;
      c.emit(load_const(dst, v), span)?;
    }
    Int(v) => {
      if let Ok(value) = i16::try_from(*v) {
        c.emit(load_smi(dst, Smi(value)), span)?;
      } else {
        let v = c.pool().int(*v)?;
        c.emit(load_const(dst, v), span)?;
      }
    }
    Nil => {
      c.emit(load_nil(dst), span)?;
    }
    Bool(v) => match *v {
      true => c.emit(load_true(dst), span)?,
      false => c.emit(load_false(dst), span)?,
    },
    String(v) => {
      let v = Str::try_intern_in(c.gc, v)?;
      let v = c.pool().str(v)?;
      c.emit(load_const(dst, v), span)?;
    }
    Record([]) => {
      c.emit(make_table_empty(dst), span)?;
    }
    Record(fields) => {
      let mut regs = vec![in c.arena];
      let mut keys = vec![in c.arena];
      for _ in fields.iter() {
        regs.push(c.reg()?);
      }
      for ((key, value), reg) in fields.iter().zip(regs.iter()) {
        assign_to(c, *reg, value)?;
        keys.push(Str::try_intern_in(c.gc, key.lexeme)?);
      }
      let desc = TableDescriptor::try_new_in(c.gc, regs[0], &keys)?;
      let desc = c.pool().table(desc)?;
      c.emit(make_table(dst, desc), span)?;
      c.free(regs[0]);
    }
    List([]) => {
      c.emit(make_list_empty(dst), span)?;
    }
    List(items) => {
      let mut regs = vec![in c.arena];
      for _ in items.iter() {
        regs.push(c.reg()?);
      }
      for (value, reg) in items.iter().zip(regs.iter()) {
        assign_to(c, *reg, value)?;
      }
      let desc = ListDescriptor::try_new_in(c.gc, regs[0], regs.len() as u8)?;
      let desc = c.pool().list(desc)?;
      c.emit(make_list(dst, desc), span)?;
      c.free(regs[0]);
    }
    Tuple([]) => {
      c.emit(make_tuple_empty(dst), span)?;
    }
    Tuple(elems) => {
      let mut regs = vec![in c.arena];
      for _ in elems.iter() {
        regs.push(c.reg()?);
      }
      for (value, reg) in elems.iter().zip(regs.iter()) {
        assign_to(c, *reg, value)?;
      }
      let desc = TupleDescriptor::try_new_in(c.gc, regs[0], regs.len() as u8)?;
      let desc = c.pool().tuple(desc)?;
      c.emit(make_tuple(dst, desc), span)?;
      c.free(regs[0]);
    }
  }

  Ok(None)
}
