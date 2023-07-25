use core::fmt::Debug;

use beef::lean::Cow;

use crate::lex::*;

pub struct Module<'arena, 'src> {
  pub src: &'src str,
  pub body: &'arena [Stmt<'arena, 'src>],
}

impl<'arena, 'src> Debug for Module<'arena, 'src> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    Debug::fmt(&self.body, f)
  }
}

#[derive(Clone, Copy, Debug)]
pub struct Ident<'src> {
  pub span: Span,
  pub lexeme: &'src str,
}

#[derive(Clone, Debug)]
pub struct Name<'src> {
  pub span: Span,
  pub lexeme: Cow<'src, str>,
}

impl<'src> Name<'src> {
  pub fn fake(span: Span, lexeme: impl Into<Cow<'src, str>>) -> Self {
    Self {
      span,
      lexeme: lexeme.into(),
    }
  }
}

impl<'src> From<Ident<'src>> for Name<'src> {
  fn from(value: Ident<'src>) -> Self {
    Name {
      span: value.span,
      lexeme: value.lexeme.into(),
    }
  }
}

#[derive(Debug)]
pub struct Stmt<'arena, 'src> {
  pub span: Span,
  pub kind: StmtKind<'arena, 'src>,
}

impl<'arena, 'src> Stmt<'arena, 'src> {
  pub fn new(kind: StmtKind<'arena, 'src>, span: Span) -> Self {
    Self { span, kind }
  }
}

// TODO: `loop`, `break`, `continue`, `return`,
// and `func` should probably always be expressions.
#[derive(Debug)]
pub enum StmtKind<'arena, 'src> {
  Let(&'arena Let<'arena, 'src>),
  Loop(&'arena Loop<'arena, 'src>),
  Break,
  Continue,
  Return(&'arena Return<'arena, 'src>),
  Func(&'arena Func<'arena, 'src>),
  Expr(&'arena Expr<'arena, 'src>),
}

#[derive(Debug)]
pub struct Let<'arena, 'src> {
  pub name: Ident<'src>,
  pub value: Option<Expr<'arena, 'src>>,
}

#[derive(Clone, Copy, Debug)]
pub enum Assign {
  Bare,
  Compound(AssignKind),
}

#[derive(Clone, Copy, Debug)]
pub enum AssignKind {
  Add,
  Sub,
  Div,
  Mul,
  Rem,
  Pow,
}

impl From<AssignKind> for BinaryOp {
  fn from(value: AssignKind) -> Self {
    match value {
      AssignKind::Add => BinaryOp::Add,
      AssignKind::Sub => BinaryOp::Sub,
      AssignKind::Div => BinaryOp::Div,
      AssignKind::Mul => BinaryOp::Mul,
      AssignKind::Rem => BinaryOp::Rem,
      AssignKind::Pow => BinaryOp::Pow,
    }
  }
}

#[derive(Debug)]
pub struct Block<'arena, 'src> {
  pub body: &'arena [Stmt<'arena, 'src>],
  pub last: Option<Expr<'arena, 'src>>,
}

#[derive(Debug)]
pub struct If<'arena, 'src> {
  pub br: &'arena [Branch<'arena, 'src>],
  pub tail: Option<Block<'arena, 'src>>,
}

#[derive(Debug)]
pub struct Branch<'arena, 'src> {
  pub cond: Expr<'arena, 'src>,
  pub body: Block<'arena, 'src>,
}

#[derive(Debug)]
pub struct Loop<'arena, 'src> {
  pub body: Block<'arena, 'src>,
}

#[derive(Debug)]
pub struct Return<'arena, 'src> {
  pub value: Option<Expr<'arena, 'src>>,
}

#[derive(Debug)]
pub struct Func<'arena, 'src> {
  pub fn_token_span: Span,
  pub name: Option<Ident<'src>>,
  pub params: &'arena [Ident<'src>],
  pub body: Block<'arena, 'src>,
}

#[derive(Clone, Copy, Debug)]
pub struct Expr<'arena, 'src> {
  pub span: Span,
  pub kind: ExprKind<'arena, 'src>,
}

impl<'arena, 'src> Expr<'arena, 'src> {
  pub fn new(kind: ExprKind<'arena, 'src>, span: Span) -> Self {
    Self { span, kind }
  }
}

#[derive(Clone, Copy, Debug)]
pub enum ExprKind<'arena, 'src> {
  Logical(&'arena Logical<'arena, 'src>),
  Binary(&'arena Binary<'arena, 'src>),
  Unary(&'arena Unary<'arena, 'src>),
  Block(&'arena Block<'arena, 'src>),
  If(&'arena If<'arena, 'src>),
  Func(&'arena Func<'arena, 'src>),
  GetVar(&'arena GetVar<'src>),
  SetVar(&'arena SetVar<'arena, 'src>),
  GetField(&'arena GetField<'arena, 'src>),
  SetField(&'arena SetField<'arena, 'src>),
  GetIndex(&'arena GetIndex<'arena, 'src>),
  SetIndex(&'arena SetIndex<'arena, 'src>),
  Call(&'arena Call<'arena, 'src>),
  Lit(&'arena Lit<'arena, 'src>),
}

#[derive(Debug)]
pub struct Logical<'arena, 'src> {
  pub op: LogicalOp,
  pub lhs: Expr<'arena, 'src>,
  pub rhs: Expr<'arena, 'src>,
}

#[derive(Clone, Copy, Debug)]
pub enum LogicalOp {
  And,
  Or,
}

#[derive(Debug)]
pub struct Binary<'arena, 'src> {
  pub op: BinaryOp,
  pub lhs: Expr<'arena, 'src>,
  pub rhs: Expr<'arena, 'src>,
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
  Ne,
  Gt,
  Ge,
  Lt,
  Le,
}

#[derive(Debug)]
pub struct Unary<'arena, 'src> {
  pub op: UnaryOp,
  pub rhs: Expr<'arena, 'src>,
}

#[derive(Clone, Copy, Debug)]
pub enum UnaryOp {
  Min,
  Not,
}

#[derive(Clone, Debug)]
pub struct GetVar<'src> {
  pub name: Ident<'src>,
}

#[derive(Clone, Debug)]
pub struct SetVar<'arena, 'src> {
  pub name: Ident<'src>,
  pub value: Expr<'arena, 'src>,
}

#[derive(Clone, Debug)]
pub enum Key<'src> {
  Int(i32),
  Ident(Ident<'src>),
}

#[derive(Clone, Debug)]
pub struct GetField<'arena, 'src> {
  pub target: &'arena Expr<'arena, 'src>,
  pub key: &'arena Key<'src>,
}

#[derive(Clone, Debug)]
pub struct SetField<'arena, 'src> {
  pub target: &'arena Expr<'arena, 'src>,
  pub key: &'arena Key<'src>,
  pub value: Expr<'arena, 'src>,
}

#[derive(Clone, Debug)]
pub struct GetIndex<'arena, 'src> {
  pub target: &'arena Expr<'arena, 'src>,
  pub index: &'arena Expr<'arena, 'src>,
}

#[derive(Clone, Debug)]
pub struct SetIndex<'arena, 'src> {
  pub target: &'arena Expr<'arena, 'src>,
  pub index: &'arena Expr<'arena, 'src>,
  pub value: Expr<'arena, 'src>,
}

#[derive(Clone, Debug)]
pub struct Call<'arena, 'src> {
  pub target: Expr<'arena, 'src>,
  pub args: &'arena [Expr<'arena, 'src>],
}

#[derive(Clone, Debug)]
pub enum Lit<'arena, 'src> {
  Float(f64),
  Int(i32),
  Nil,
  Bool(bool),
  String(&'src str),
  Record(&'arena [(Ident<'src>, Expr<'arena, 'src>)]),
  List(&'arena [Expr<'arena, 'src>]),
  Tuple(&'arena [Expr<'arena, 'src>]),
}