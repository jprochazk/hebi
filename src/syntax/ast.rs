#![allow(clippy::needless_lifetimes)]

use std::collections::BTreeMap;
use std::fmt::Display;
use std::ops::{Deref, DerefMut};

use beef::lean::Cow;

use crate::span::{Span, Spanned};

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ident<'src>(Spanned<Cow<'src, str>>);

impl<'src> Ident<'src> {
  pub fn new(span: impl Into<Span>, lexeme: Cow<'src, str>) -> Self {
    Self(Spanned::new(span, lexeme))
  }

  pub fn lexeme(&self) -> Cow<'src, str> {
    self.0.deref().clone()
  }
}

impl<'src> Deref for Ident<'src> {
  type Target = Spanned<Cow<'src, str>>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<'src> DerefMut for Ident<'src> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Symbol<'src>(Spanned<Cow<'src, str>>);

impl<'src> Symbol<'src> {
  pub fn new(span: impl Into<Span>, lexeme: Cow<'src, str>) -> Self {
    Self(Spanned::new(span, lexeme))
  }

  /// Strips the leading `@` and returns the result as an identifier.
  ///
  /// This copies the string.
  pub fn ident<'a>(&'a self) -> Ident<'src> {
    let span = self.0.span;
    Ident::new(span, Cow::from(self.0.deref()[1..].to_string()))
  }

  pub fn which(&self) -> Option<Meta> {
    Meta::parse(&self.0.deref()[1..])
  }

  pub fn lexeme(&self) -> Cow<'src, str> {
    self.0.deref().clone()
  }
}

impl<'src> Deref for Symbol<'src> {
  type Target = Spanned<Cow<'src, str>>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<'src> DerefMut for Symbol<'src> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

pub type Map<K, V> = BTreeMap<K, V>;

#[cfg_attr(test, derive(Debug))]
pub struct Module<'src> {
  pub body: Vec<Stmt<'src>>,
}

impl<'src> Module<'src> {
  pub fn new() -> Self {
    Self { body: vec![] }
  }
}

impl<'src> Default for Module<'src> {
  fn default() -> Self {
    Self::new()
  }
}

pub type Stmt<'src> = Spanned<StmtKind<'src>>;

#[cfg_attr(test, derive(Debug))]
pub enum StmtKind<'src> {
  Var(Box<Var<'src>>),
  If(Box<If<'src>>),
  Loop(Box<Loop<'src>>),
  Ctrl(Box<Ctrl<'src>>),
  Func(Box<Func<'src>>),
  Class(Box<Class<'src>>),
  Expr(Box<Expr<'src>>),
  Pass,
  Print(Box<Print<'src>>),
  Import(Box<Import<'src>>),
}

#[cfg_attr(test, derive(Debug))]
pub enum Import<'src> {
  Module {
    path: Vec<Ident<'src>>,
    alias: Option<Ident<'src>>,
  },
  Symbols {
    path: Vec<Ident<'src>>,
    symbols: Vec<ImportSymbol<'src>>,
  },
}

#[cfg_attr(test, derive(Debug))]
pub struct ImportSymbol<'src> {
  pub name: Ident<'src>,
  pub alias: Option<Ident<'src>>,
}

#[cfg_attr(test, derive(Debug))]
pub struct Func<'src> {
  pub name: Ident<'src>,
  pub params: Params<'src>,
  pub body: Vec<Stmt<'src>>,
  pub has_yield: bool,
}

#[cfg_attr(test, derive(Debug))]
#[derive(Default)]
pub struct Params<'src> {
  pub has_self: bool,
  pub pos: Vec<Param<'src>>,
}

impl<'src> Params<'src> {
  pub fn contains(&self, param: &Ident<'src>) -> bool {
    self.pos.iter().any(|v| v.name.as_ref() == param.as_ref())
  }
}

#[cfg_attr(test, derive(Debug))]
pub struct Param<'src> {
  pub name: Ident<'src>,
  pub default: Option<Expr<'src>>,
}

impl<'src> Param<'src> {
  pub fn span(&self) -> Span {
    match &self.default {
      Some(default) => self.name.span.join(default.span),
      None => self.name.span,
    }
  }
}

#[cfg_attr(test, derive(Debug))]
pub struct Class<'src> {
  pub name: Ident<'src>,
  pub parent: Option<Ident<'src>>,
  pub members: ClassMembers<'src>,
}

#[cfg_attr(test, derive(Debug))]
pub struct ClassMembers<'src> {
  pub fields: Vec<Field<'src>>,
  pub methods: Vec<Func<'src>>,
  pub meta: Vec<(Meta, Func<'src>)>,
}

impl<'src> ClassMembers<'src> {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self {
    Self {
      fields: vec![],
      methods: vec![],
      meta: vec![],
    }
  }
}

#[cfg_attr(test, derive(Debug))]
pub struct Field<'src> {
  pub name: Ident<'src>,
  pub default: Expr<'src>,
}

impl<'src> Field<'src> {
  pub fn span(&self) -> Span {
    self.name.span.join(self.default.span)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Meta {
  Str,

  Init,

  Add,
  Sub,
  Mul,
  Div,
  Rem,
  Pow,

  Cmp,

  Index,

  Iter,
  Next,
  Done,

  Enter,
  Leave,
}

impl Meta {
  pub fn parse(s: &str) -> Option<Self> {
    let v = match s {
      "str" => Self::Str,
      "init" => Self::Init,
      "add" => Self::Add,
      "sub" => Self::Sub,
      "mul" => Self::Mul,
      "div" => Self::Div,
      "rem" => Self::Rem,
      "pow" => Self::Pow,
      "cmp" => Self::Cmp,
      "index" => Self::Index,
      "iter" => Self::Iter,
      "next" => Self::Next,
      "done" => Self::Done,
      "enter" => Self::Enter,
      "leave" => Self::Leave,
      _ => return None,
    };
    Some(v)
  }

  pub fn arity(&self) -> Option<usize> {
    match self {
      Meta::Init => None,
      Meta::Add
      | Meta::Sub
      | Meta::Mul
      | Meta::Div
      | Meta::Rem
      | Meta::Pow
      | Meta::Cmp
      | Meta::Index => Some(1),
      Meta::Str | Meta::Iter | Meta::Next | Meta::Done | Meta::Enter | Meta::Leave => Some(0),
    }
  }

  pub fn param_names(&self) -> &'static [&'static str] {
    match self {
      Meta::Add | Meta::Sub | Meta::Mul | Meta::Div | Meta::Rem | Meta::Pow | Meta::Cmp => {
        &["other"]
      }
      Meta::Index => &["key"],
      Meta::Str | Meta::Init | Meta::Iter | Meta::Next | Meta::Done | Meta::Enter | Meta::Leave => {
        &[]
      }
    }
  }

  pub fn as_str(&self) -> &'static str {
    match self {
      Self::Str => "str",
      Self::Init => "init",
      Self::Add => "add",
      Self::Sub => "sub",
      Self::Mul => "mul",
      Self::Div => "div",
      Self::Rem => "rem",
      Self::Pow => "pow",
      Self::Cmp => "cmp",
      Self::Index => "index",
      Self::Iter => "iter",
      Self::Next => "next",
      Self::Done => "done",
      Self::Enter => "enter",
      Self::Leave => "leave",
    }
  }
}

#[cfg_attr(test, derive(Debug))]
pub enum Loop<'src> {
  For(For<'src>),
  While(While<'src>),
  Infinite(Infinite<'src>),
}

#[cfg_attr(test, derive(Debug))]
pub struct For<'src> {
  pub item: Ident<'src>,
  pub iter: ForIter<'src>,
  pub body: Vec<Stmt<'src>>,
}

#[cfg_attr(test, derive(Debug))]
pub enum ForIter<'src> {
  Range(IterRange<'src>),
  Expr(Expr<'src>),
}

#[cfg_attr(test, derive(Debug))]
pub struct IterRange<'src> {
  pub start: Expr<'src>,
  pub end: Expr<'src>,
  pub inclusive: bool,
}

impl<'src> IterRange<'src> {
  pub fn span(&self) -> Span {
    self.start.span.join(self.end.span)
  }
}

#[cfg_attr(test, derive(Debug))]
pub struct While<'src> {
  pub cond: Expr<'src>,
  pub body: Vec<Stmt<'src>>,
}

#[cfg_attr(test, derive(Debug))]
pub struct Infinite<'src> {
  pub body: Vec<Stmt<'src>>,
}

#[cfg_attr(test, derive(Debug))]
pub struct Print<'src> {
  pub values: Vec<Expr<'src>>,
}

pub type Expr<'src> = Spanned<ExprKind<'src>>;

#[cfg_attr(test, derive(Debug))]
#[derive(Clone)]
pub enum ExprKind<'src> {
  Literal(Box<Literal<'src>>),
  Binary(Box<Binary<'src>>),
  Unary(Box<Unary<'src>>),
  GetVar(Box<GetVar<'src>>),
  SetVar(Box<SetVar<'src>>),
  GetField(Box<GetField<'src>>),
  SetField(Box<SetField<'src>>),
  GetIndex(Box<GetIndex<'src>>),
  SetIndex(Box<SetIndex<'src>>),
  Call(Box<Call<'src>>),
  GetSelf,
  GetSuper,
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone)]
pub enum Literal<'src> {
  None,
  Int(i32),
  Float(f64),
  Bool(bool),
  String(Cow<'src, str>),
  List(Vec<Expr<'src>>),
  Table(Vec<(Expr<'src>, Expr<'src>)>),
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone)]
pub struct Binary<'src> {
  pub op: BinaryOp,
  pub left: Expr<'src>,
  pub right: Expr<'src>,
}

#[derive(Clone, Copy, Debug)]
pub enum BinaryOp {
  Add,
  Sub,
  Div,
  Mul,
  Rem,
  Pow,
  Eq,
  Neq,
  More,
  MoreEq,
  Less,
  LessEq,
  And,
  Or,
  Maybe,
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone)]
pub struct Unary<'src> {
  pub op: UnaryOp,
  pub right: Expr<'src>,
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone, Copy)]
pub enum UnaryOp {
  Plus,
  Minus,
  Not,
  Opt,
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone)]
pub struct GetVar<'src> {
  pub name: Ident<'src>,
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone)]
pub struct SetVar<'src> {
  pub target: GetVar<'src>,
  pub value: Expr<'src>,
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone)]
pub struct GetField<'src> {
  pub target: Expr<'src>,
  pub name: Ident<'src>,
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone)]
pub struct SetField<'src> {
  pub target: GetField<'src>,
  pub value: Expr<'src>,
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone)]
pub struct GetIndex<'src> {
  pub target: Expr<'src>,
  pub key: Expr<'src>,
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone)]
pub struct SetIndex<'src> {
  pub target: GetIndex<'src>,
  pub value: Expr<'src>,
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone, Copy)]
pub enum AssignOp {
  Add,
  Sub,
  Div,
  Mul,
  Rem,
  Pow,
  Maybe,
}

impl From<AssignOp> for BinaryOp {
  fn from(value: AssignOp) -> Self {
    match value {
      AssignOp::Add => BinaryOp::Add,
      AssignOp::Sub => BinaryOp::Sub,
      AssignOp::Div => BinaryOp::Div,
      AssignOp::Mul => BinaryOp::Mul,
      AssignOp::Rem => BinaryOp::Rem,
      AssignOp::Pow => BinaryOp::Pow,
      AssignOp::Maybe => BinaryOp::Maybe,
    }
  }
}

#[derive(Clone, Copy)]
pub enum AssignKind {
  Op(Option<AssignOp>),
  Decl,
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone)]
pub struct Yield<'src> {
  pub value: Option<Expr<'src>>,
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone)]
pub struct Return<'src> {
  pub value: Option<Expr<'src>>,
}

#[cfg_attr(test, derive(Debug))]
#[derive(Clone)]
pub struct Call<'src> {
  pub target: Expr<'src>,
  pub args: Vec<Expr<'src>>,
}

#[cfg_attr(test, derive(Debug))]
pub struct Var<'src> {
  pub name: Ident<'src>,
  pub value: Expr<'src>,
}

#[cfg_attr(test, derive(Debug))]
pub struct If<'src> {
  pub branches: Vec<Branch<'src>>,
  pub default: Option<Vec<Stmt<'src>>>,
}

#[cfg_attr(test, derive(Debug))]
pub struct Branch<'src> {
  pub cond: Expr<'src>,
  pub body: Vec<Stmt<'src>>,
}

#[cfg_attr(test, derive(Debug))]
pub enum Ctrl<'src> {
  Return(Return<'src>),
  Yield(Yield<'src>),
  Continue,
  Break,
}

pub fn import_module_stmt<'src>(
  s: impl Into<Span>,
  path: Vec<Ident<'src>>,
  alias: Option<Ident<'src>>,
) -> Stmt<'src> {
  Stmt::new(
    s,
    StmtKind::Import(Box::new(Import::Module { path, alias })),
  )
}

pub fn import_symbols_stmt<'src>(
  s: impl Into<Span>,
  path: Vec<Ident<'src>>,
  symbols: Vec<ImportSymbol<'src>>,
) -> Stmt<'src> {
  Stmt::new(
    s,
    StmtKind::Import(Box::new(Import::Symbols { path, symbols })),
  )
}

pub fn if_stmt<'src>(
  s: impl Into<Span>,
  branches: Vec<Branch<'src>>,
  default: Option<Vec<Stmt<'src>>>,
) -> Stmt<'src> {
  Stmt::new(s, StmtKind::If(Box::new(If { branches, default })))
}

pub fn branch<'src>(cond: Expr<'src>, body: Vec<Stmt<'src>>) -> Branch<'src> {
  Branch { cond, body }
}

pub fn return_stmt(s: impl Into<Span>, value: Option<Expr>) -> Stmt {
  Stmt::new(s, StmtKind::Ctrl(Box::new(Ctrl::Return(Return { value }))))
}

pub fn yield_stmt(inner: Spanned<Yield>) -> Stmt {
  Stmt::new(
    inner.span,
    StmtKind::Ctrl(Box::new(Ctrl::Yield(inner.into_inner()))),
  )
}

pub fn continue_stmt<'src>(s: impl Into<Span>) -> Stmt<'src> {
  Stmt::new(s, StmtKind::Ctrl(Box::new(Ctrl::Continue)))
}

pub fn break_stmt<'src>(s: impl Into<Span>) -> Stmt<'src> {
  Stmt::new(s, StmtKind::Ctrl(Box::new(Ctrl::Break)))
}

pub fn pass_stmt<'src>(s: impl Into<Span>) -> Stmt<'src> {
  Stmt::new(s, StmtKind::Pass)
}

pub fn print_stmt(s: impl Into<Span>, values: Vec<Expr>) -> Stmt {
  Stmt::new(s, StmtKind::Print(Box::new(Print { values })))
}

pub fn expr_binary<'src>(
  s: impl Into<Span>,
  op: BinaryOp,
  left: Expr<'src>,
  right: Expr<'src>,
) -> Expr<'src> {
  Expr::new(s, ExprKind::Binary(Box::new(Binary { op, left, right })))
}

pub fn expr_unary(s: impl Into<Span>, op: UnaryOp, right: Expr) -> Expr {
  Expr::new(s, ExprKind::Unary(Box::new(Unary { op, right })))
}

pub fn expr_call<'src>(
  s: impl Into<Span>,
  target: Expr<'src>,
  args: Vec<Expr<'src>>,
) -> Expr<'src> {
  Expr::new(s, ExprKind::Call(Box::new(Call { target, args })))
}

pub fn expr_get_field<'src>(
  s: impl Into<Span>,
  target: Expr<'src>,
  name: Ident<'src>,
) -> Expr<'src> {
  Expr::new(s, ExprKind::GetField(Box::new(GetField { target, name })))
}

pub fn expr_get_index<'src>(s: impl Into<Span>, target: Expr<'src>, key: Expr<'src>) -> Expr<'src> {
  Expr::new(s, ExprKind::GetIndex(Box::new(GetIndex { target, key })))
}

pub fn expr_list(s: impl Into<Span>, items: Vec<Expr>) -> Expr {
  Expr::new(s, ExprKind::Literal(Box::new(Literal::List(items))))
}

pub fn ident_key(v: Ident) -> Expr {
  Expr::new(
    v.span,
    ExprKind::Literal(Box::new(Literal::String(v.lexeme()))),
  )
}

pub fn expr_table<'src>(s: impl Into<Span>, items: Vec<(Expr<'src>, Expr<'src>)>) -> Expr<'src> {
  Expr::new(s, ExprKind::Literal(Box::new(Literal::Table(items))))
}

pub fn expr_get_var(name: Ident) -> Expr {
  Expr::new(name.span, ExprKind::GetVar(Box::new(GetVar { name })))
}

pub fn expr_get_self<'src>(s: impl Into<Span>) -> Expr<'src> {
  Expr::new(s, ExprKind::GetSelf)
}

pub fn expr_get_super<'src>(s: impl Into<Span>) -> Expr<'src> {
  Expr::new(s, ExprKind::GetSuper)
}

pub fn expr_stmt(expr: Expr) -> Stmt {
  Stmt::new(expr.span, StmtKind::Expr(Box::new(expr)))
}

pub fn var_stmt<'src>(name: Ident<'src>, value: Expr<'src>) -> Stmt<'src> {
  Stmt::new(
    name.span.start..value.span.end,
    StmtKind::Var(Box::new(Var { name, value })),
  )
}

pub fn func_stmt(s: impl Into<Span>, func: Func) -> Stmt {
  Stmt::new(s, StmtKind::Func(Box::new(func)))
}

pub fn func<'src>(
  name: Ident<'src>,
  params: Params<'src>,
  body: Vec<Stmt<'src>>,
  has_yield: bool,
) -> Func<'src> {
  Func {
    name,
    params,
    body,
    has_yield,
  }
}

pub fn class_stmt<'src>(
  s: impl Into<Span>,
  name: Ident<'src>,
  parent: Option<Ident<'src>>,
  members: ClassMembers<'src>,
) -> Stmt<'src> {
  Stmt::new(
    s,
    StmtKind::Class(Box::new(Class {
      name,
      parent,
      members,
    })),
  )
}

pub fn assign<'src>(target: Expr<'src>, kind: AssignKind, value: Expr<'src>) -> Option<Stmt<'src>> {
  let span = Span::from(target.span.start..value.span.end);
  match kind {
    AssignKind::Decl => {
      let name = match target.into_inner() {
        ExprKind::GetVar(target) => target.name,
        _ => return None,
      };
      Some(var_stmt(name, value))
    }
    AssignKind::Op(op) => {
      let assign = match target.into_inner() {
        ExprKind::GetVar(target) => ExprKind::SetVar(Box::new(SetVar {
          value: desugar_assign(span, &*target, op, value),
          target: *target,
        })),
        ExprKind::GetField(target) => ExprKind::SetField(Box::new(SetField {
          value: desugar_assign(span, &*target, op, value),
          target: *target,
        })),
        ExprKind::GetIndex(target) => ExprKind::SetIndex(Box::new(SetIndex {
          value: desugar_assign(span, &*target, op, value),
          target: *target,
        })),
        _ => return None,
      };
      Some(expr_stmt(Expr::new(span, assign)))
    }
  }
}

fn desugar_assign<'src, T>(
  span: impl Into<Span>,
  target: &T,
  op: Option<AssignOp>,
  value: Expr<'src>,
) -> Expr<'src>
where
  T: Clone,
  ExprKind<'src>: From<T>,
{
  let span = span.into();
  match op {
    Some(op) => expr_binary(
      span,
      op.into(),
      Expr::new(span, ExprKind::from(target.clone())),
      value,
    ),
    None => value,
  }
}

impl<'src> From<GetVar<'src>> for ExprKind<'src> {
  fn from(value: GetVar<'src>) -> Self {
    ExprKind::GetVar(Box::new(value))
  }
}

impl<'src> From<GetField<'src>> for ExprKind<'src> {
  fn from(value: GetField<'src>) -> Self {
    ExprKind::GetField(Box::new(value))
  }
}

impl<'src> From<GetIndex<'src>> for ExprKind<'src> {
  fn from(value: GetIndex<'src>) -> Self {
    ExprKind::GetIndex(Box::new(value))
  }
}

pub fn loop_stmt(s: impl Into<Span>, body: Vec<Stmt>) -> Stmt {
  Stmt::new(
    s,
    StmtKind::Loop(Box::new(Loop::Infinite(Infinite { body }))),
  )
}

pub fn while_loop_stmt<'src>(
  s: impl Into<Span>,
  cond: Expr<'src>,
  body: Vec<Stmt<'src>>,
) -> Stmt<'src> {
  Stmt::new(
    s,
    StmtKind::Loop(Box::new(Loop::While(While { cond, body }))),
  )
}

pub fn for_loop_stmt<'src>(
  s: impl Into<Span>,
  item: Ident<'src>,
  iter: ForIter<'src>,
  body: Vec<Stmt<'src>>,
) -> Stmt<'src> {
  Stmt::new(
    s,
    StmtKind::Loop(Box::new(Loop::For(For { item, iter, body }))),
  )
}

pub mod lit {
  use super::*;
  use crate::ctx::Context;
  use crate::error::Result;
  use crate::span::Span;

  pub fn none<'src>(_: &Context, s: impl Into<Span>) -> Expr<'src> {
    let s = s.into();
    Expr::new(s, ExprKind::Literal(Box::new(Literal::None)))
  }

  pub fn bool<'src>(cx: &Context, s: impl Into<Span>, lexeme: &str) -> Result<Expr<'src>> {
    let s = s.into();
    let v = match lexeme {
      "true" => true,
      "false" => false,
      _ => return Err(cx.error("bool is only ever `true` or `false`", s)),
    };
    Ok(Expr::new(s, ExprKind::Literal(Box::new(Literal::Bool(v)))))
  }

  pub fn int<'src>(cx: &Context, s: impl Into<Span>, lexeme: &'src str) -> Result<Expr<'src>> {
    let s = s.into();
    let value = lexeme
      .parse::<i64>()
      .map_err(|e| cx.error(format!("invalid number {e}"), s))?;
    let lit = if value < (i32::MIN as i64) || (i32::MAX as i64) < value {
      // TODO: bigint?
      Literal::Float(value as f64)
    } else {
      Literal::Int(value as i32)
    };
    Ok(Expr::new(s, ExprKind::Literal(Box::new(lit))))
  }

  pub fn float<'src>(cx: &Context, s: impl Into<Span>, lexeme: &'src str) -> Result<Expr<'src>> {
    let s = s.into();
    let value = lexeme
      .parse()
      .map_err(|e| cx.error(format!("invalid number {e}"), s))?;
    Ok(Expr::new(
      s,
      ExprKind::Literal(Box::new(Literal::Float(value))),
    ))
  }

  pub fn str<'src>(s: impl Into<Span>, lexeme: &'src str) -> Option<Expr<'src>> {
    let s = s.into();
    let lexeme = lexeme.strip_prefix('"').unwrap_or(lexeme);
    let lexeme = lexeme.strip_suffix('"').unwrap_or(lexeme);
    let mut lexeme = lexeme.to_string();
    unescape_in_place(&mut lexeme)?;
    Some(Expr::new(
      s,
      ExprKind::Literal(Box::new(Literal::String(Cow::from(lexeme)))),
    ))
  }

  // Adapted from https://docs.rs/snailquote/0.3.0/x86_64-pc-windows-msvc/src/snailquote/lib.rs.html.
  /// Unescapes the given string in-place. Returns `None` if the string contains
  /// an invalid escape sequence.
  fn unescape_in_place(s: &mut String) -> Option<()> {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
      if ch == '\\' {
        if let Some(next) = chars.next() {
          let escape = match next {
            'a' => Some('\u{07}'),
            'b' => Some('\u{08}'),
            'v' => Some('\u{0B}'),
            'f' => Some('\u{0C}'),
            'n' => Some('\n'),
            'r' => Some('\r'),
            't' => Some('\t'),
            '\'' => Some('\''),
            '"' => Some('"'),
            '\\' => Some('\\'),
            'e' | 'E' => Some('\u{1B}'),
            'x' => Some(parse_hex_code(&mut chars)?),
            'u' => Some(parse_unicode(&mut chars)?),
            _ => None,
          };
          match escape {
            Some(esc) => {
              out.push(esc);
            }
            None => {
              out.push(ch);
              out.push(next);
            }
          }
        }
      } else {
        out.push(ch);
      }
    }
    *s = out;
    Some(())
  }

  fn parse_hex_code<I>(chars: &mut I) -> Option<char>
  where
    I: Iterator<Item = char>,
  {
    let digits = [
      u8::try_from(chars.next()?).ok()?,
      u8::try_from(chars.next()?).ok()?,
    ];
    let digits = std::str::from_utf8(&digits[..]).ok()?;
    let c = u32::from_str_radix(digits, 16).ok()?;
    char::from_u32(c)
  }

  // Adapted from https://docs.rs/snailquote/0.3.0/x86_64-pc-windows-msvc/src/snailquote/lib.rs.html.
  fn parse_unicode<I>(chars: &mut I) -> Option<char>
  where
    I: Iterator<Item = char>,
  {
    match chars.next() {
      Some('{') => {}
      _ => {
        return None;
      }
    }

    let unicode_seq: String = chars.take_while(|&c| c != '}').collect();

    u32::from_str_radix(&unicode_seq, 16)
      .ok()
      .and_then(char::from_u32)
  }
}

impl<'src> Display for Ident<'src> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl<'src> Display for Symbol<'src> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "@{}", self.0)
  }
}
