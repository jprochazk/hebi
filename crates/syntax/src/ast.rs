use std::collections::BTreeMap;
use std::ops::Range;

use beef::lean::Cow;
use span::Spanned;

pub type Ident<'src> = Spanned<Cow<'src, str>>;
pub type Map<K, V> = BTreeMap<K, V>;

#[cfg_attr(test, derive(Debug))]
pub struct Module<'src> {
  pub imports: Vec<Import<'src>>,
  pub body: Vec<Stmt<'src>>,
}

impl<'src> Module<'src> {
  pub fn new() -> Self {
    Self {
      imports: vec![],
      body: vec![],
    }
  }
}

impl<'src> Default for Module<'src> {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg_attr(test, derive(Debug))]
pub struct Import<'src> {
  pub path: Vec<Ident<'src>>,
  pub alias: Option<Ident<'src>>,
}

impl<'src> Import<'src> {
  pub fn normal(path: Vec<Ident<'src>>) -> Self {
    Import { path, alias: None }
  }

  pub fn alias(path: Vec<Ident<'src>>, alias: Ident<'src>) -> Self {
    Import {
      path,
      alias: Some(alias),
    }
  }
}

pub type Stmt<'src> = Spanned<StmtKind<'src>>;

#[cfg_attr(test, derive(Debug))]
pub enum StmtKind<'src> {
  If(Box<If<'src>>),
  Loop(Box<Loop<'src>>),
  Ctrl(Box<Ctrl<'src>>),
  Func(Box<Func<'src>>),
  Class(Box<Class<'src>>),
  Expr(Box<Expr<'src>>),
}

#[cfg_attr(test, derive(Debug))]
pub struct Func<'src> {
  pub name: Ident<'src>,
  pub params: Vec<Ident<'src>>,
  pub body: Vec<Stmt<'src>>,
  pub has_yield: bool,
}

#[cfg_attr(test, derive(Debug))]
pub struct Class<'src> {
  pub name: Ident<'src>,
  pub funcs: Vec<Func<'src>>,
}

#[cfg_attr(test, derive(Debug))]
pub enum Loop<'src> {
  For(For<'src>),
  While(While<'src>),
  Infinite(Infinite<'src>),
}

#[cfg_attr(test, derive(Debug))]
pub struct For<'src> {
  pub item_var: Ident<'src>,
  pub iter: Expr<'src>,
  pub body: Vec<Stmt<'src>>,
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

pub type Expr<'src> = Spanned<ExprKind<'src>>;

#[cfg_attr(test, derive(Debug))]
pub enum ExprKind<'src> {
  Literal(Box<Literal<'src>>),
  Binary(Box<Binary<'src>>),
  Unary(Box<Unary<'src>>),
  GetVar(Box<GetVar<'src>>),
  SetVar(Box<SetVar<'src>>),
  GetField(Box<GetField<'src>>),
  SetField(Box<SetField<'src>>),
  Call(Box<Call<'src>>),
}

#[cfg_attr(test, derive(Debug))]
pub enum Literal<'src> {
  Null,
  Number(f64),
  Bool(bool),
  String(Cow<'src, str>),
  Array(Vec<Expr<'src>>),
  Object(Vec<(Expr<'src>, Expr<'src>)>),
}

#[cfg_attr(test, derive(Debug))]
pub struct Binary<'src> {
  pub op: BinaryOp,
  pub left: Expr<'src>,
  pub right: Expr<'src>,
}

#[cfg_attr(test, derive(Debug))]
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
pub struct Unary<'src> {
  pub op: UnaryOp,
  pub right: Expr<'src>,
}

#[cfg_attr(test, derive(Debug))]
pub enum UnaryOp {
  Minus,
  Not,
}

#[cfg_attr(test, derive(Debug))]
pub struct GetVar<'src> {
  pub name: Ident<'src>,
}

#[cfg_attr(test, derive(Debug))]
pub struct SetVar<'src> {
  pub target: GetVar<'src>,
  pub op: Option<AssignOp>,
  pub value: Expr<'src>,
}

#[cfg_attr(test, derive(Debug))]
pub struct GetField<'src> {
  pub target: Expr<'src>,
  pub key: Expr<'src>,
}

#[cfg_attr(test, derive(Debug))]
pub struct SetField<'src> {
  pub target: GetField<'src>,
  pub op: Option<AssignOp>,
  pub value: Expr<'src>,
}

#[cfg_attr(test, derive(Debug))]
pub enum AssignOp {
  Add,
  Sub,
  Div,
  Mul,
  Rem,
  Pow,
  Maybe,
}

#[cfg_attr(test, derive(Debug))]
pub struct Call<'src> {
  pub target: Expr<'src>,
  pub args: Vec<Expr<'src>>,
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
  Return(Option<Expr<'src>>),
  Yield(Expr<'src>),
  Continue,
  Break,
}

pub fn stmt_if<'src>(
  s: Range<usize>,
  branches: Vec<Branch<'src>>,
  default: Option<Vec<Stmt<'src>>>,
) -> Stmt<'src> {
  Stmt::new(s, StmtKind::If(Box::new(If { branches, default })))
}

pub fn branch<'src>(cond: Expr<'src>, body: Vec<Stmt<'src>>) -> Branch<'src> {
  Branch { cond, body }
}

pub fn stmt_expr(e: Expr) -> Stmt {
  Stmt::new(e.span, StmtKind::Expr(Box::new(e)))
}

pub fn stmt_return(s: Range<usize>, v: Option<Expr>) -> Stmt {
  Stmt::new(s, StmtKind::Ctrl(Box::new(Ctrl::Return(v))))
}

pub fn stmt_yield(s: Range<usize>, v: Expr) -> Stmt {
  Stmt::new(s, StmtKind::Ctrl(Box::new(Ctrl::Yield(v))))
}

pub fn stmt_continue<'src>(s: Range<usize>) -> Stmt<'src> {
  Stmt::new(s, StmtKind::Ctrl(Box::new(Ctrl::Continue)))
}

pub fn stmt_break<'src>(s: Range<usize>) -> Stmt<'src> {
  Stmt::new(s, StmtKind::Ctrl(Box::new(Ctrl::Break)))
}
