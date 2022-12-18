use std::collections::BTreeMap;

use beef::lean::Cow;
use span::Spanned;

pub type Ident<'src> = Spanned<Cow<'src, str>>;
pub type Map<K, V> = BTreeMap<K, V>;

pub struct Module<'src> {
  pub imports: Vec<Import<'src>>,
  pub body: Vec<Stmt<'src>>,
}

pub struct Import<'src> {
  pub path: Vec<Ident<'src>>,
  pub alias: Option<Ident<'src>>,
}

pub type Stmt<'src> = Spanned<StmtKind<'src>>;

pub enum StmtKind<'src> {
  Func(Box<Func<'src>>),
  Class(Box<Class<'src>>),
  Loop(Box<Loop<'src>>),
  Expr(Box<Expr<'src>>),
}

pub struct Func<'src> {
  pub name: Ident<'src>,
  pub params: Vec<Ident<'src>>,
  pub body: Vec<Stmt<'src>>,
  pub last_expr: Option<Expr<'src>>,
  pub has_yield: bool,
}

pub struct Class<'src> {
  pub name: Ident<'src>,
  pub funcs: Vec<Func<'src>>,
}

pub enum Loop<'src> {
  For(For<'src>),
  While(While<'src>),
  Infinite(Infinite<'src>),
}

pub struct For<'src> {
  pub item_var: Ident<'src>,
  pub iter: Expr<'src>,
  pub body: Vec<Stmt<'src>>,
}

pub struct While<'src> {
  pub cond: Expr<'src>,
  pub body: Vec<Stmt<'src>>,
}

pub struct Infinite<'src> {
  pub body: Vec<Stmt<'src>>,
}

pub type Expr<'src> = Spanned<ExprKind<'src>>;

pub enum ExprKind<'src> {
  Literal(Box<Literal<'src>>),
  Binary(Box<Binary<'src>>),
  Unary(Box<Unary<'src>>),
  GetVar(Box<GetVar<'src>>),
  SetVar(Box<SetVar<'src>>),
  GetField(Box<GetField<'src>>),
  SetField(Box<SetField<'src>>),
  Call(Box<Call<'src>>),
  If(Box<If<'src>>),
  Ctrl(Box<Ctrl<'src>>),
}

pub enum Literal<'src> {
  Null,
  Number(f64),
  Bool(bool),
  String(Cow<'src, str>),
  Array(Vec<Expr<'src>>),
  Object(Vec<(Expr<'src>, Expr<'src>)>),
}

pub struct Binary<'src> {
  pub op: BinaryOp,
  pub left: Expr<'src>,
  pub right: Expr<'src>,
}

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

pub struct Unary<'src> {
  pub op: UnaryOp,
  pub right: Expr<'src>,
}

pub enum UnaryOp {
  Minus,
  Not,
}

pub struct GetVar<'src> {
  pub name: Ident<'src>,
}

pub struct SetVar<'src> {
  pub target: GetVar<'src>,
  pub op: Option<AssignOp>,
  pub value: Expr<'src>,
}

pub struct GetField<'src> {
  pub target: Expr<'src>,
  pub key: Expr<'src>,
}

pub struct SetField<'src> {
  pub target: GetField<'src>,
  pub op: Option<AssignOp>,
  pub value: Expr<'src>,
}

pub enum AssignOp {
  Add,
  Sub,
  Div,
  Mul,
  Rem,
  Pow,
  Maybe,
}

pub struct Call<'src> {
  pub target: Expr<'src>,
  pub args: Vec<Expr<'src>>,
}

pub struct If<'src> {
  pub branches: Vec<Branch<'src>>,
  pub default: Option<Branch<'src>>,
}

pub struct Branch<'src> {
  pub cond: Expr<'src>,
  pub body: Vec<Stmt<'src>>,
  pub last_expr: Option<Expr<'src>>,
}

pub enum Ctrl<'src> {
  Yield(Expr<'src>),
  Return(Option<Expr<'src>>),
  Break,
  Continue,
}
