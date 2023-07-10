use crate::lex::*;

pub type Module<'arena, 'src> = &'arena [Stmt<'arena, 'src>];

#[derive(Clone, Copy, Debug)]
pub struct Ident<'src> {
  pub span: Span,
  pub lexeme: &'src str,
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

#[derive(Debug)]
pub enum StmtKind<'arena, 'src> {
  Let(&'arena Let<'arena, 'src>),
  Block(&'arena Block<'arena, 'src>),
  If(&'arena If<'arena, 'src>),
  Loop(&'arena Loop<'arena, 'src>),
  Break,
  Continue,
  Func(&'arena Func<'arena, 'src>),
  Expr(&'arena Expr<'arena, 'src>),
}

#[derive(Debug)]
pub struct Let<'arena, 'src> {
  pub name: Ident<'src>,
  pub value: Option<Expr<'arena, 'src>>,
}

#[derive(Clone, Copy, Debug)]
pub enum AssignKind {
  Bare,
  Add,
  Sub,
  Div,
  Mul,
  Rem,
  Pow,
}

#[derive(Debug)]
pub struct Block<'arena, 'src> {
  pub body: &'arena [Stmt<'arena, 'src>],
  pub last: Option<Expr<'arena, 'src>>,
}

#[derive(Debug)]
pub struct If<'arena, 'src> {
  pub cond: Expr<'arena, 'src>,
  pub body: Block<'arena, 'src>,
  pub tail: Option<Block<'arena, 'src>>,
}

#[derive(Debug)]
pub struct Loop<'arena, 'src> {
  pub body: Block<'arena, 'src>,
}

#[derive(Debug)]
pub struct Func<'arena, 'src> {
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
pub struct Binary<'arena, 'src> {
  pub op: BinaryOp,
  pub lhs: Expr<'arena, 'src>,
  pub rhs: Expr<'arena, 'src>,
}

#[derive(Debug)]
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
  And,
  Or,
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
  pub kind: AssignKind,
}

#[derive(Clone, Debug)]
pub enum Key<'src> {
  Int(usize),
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
  pub kind: AssignKind,
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
  pub kind: AssignKind,
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
  None,
  Bool(bool),
  String(&'src str),
  Record(&'arena [(Ident<'src>, Expr<'arena, 'src>)]),
  List(&'arena [Expr<'arena, 'src>]),
  Tuple(&'arena [Expr<'arena, 'src>]),
}
