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
  Var(Box<Var<'src>>),
  If(Box<If<'src>>),
  Loop(Box<Loop<'src>>),
  Ctrl(Box<Ctrl<'src>>),
  Func(Box<Func<'src>>),
  Class(Box<Class<'src>>),
  Expr(Box<Expr<'src>>),
  Pass,
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
  Plus,
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

pub enum AssignKind {
  Op(Option<AssignOp>),
  Decl,
}

#[cfg_attr(test, derive(Debug))]
pub struct Call<'src> {
  pub target: Expr<'src>,
  pub args: Args<'src>,
}

#[cfg_attr(test, derive(Debug))]
pub struct Args<'src> {
  pub pos: Vec<Expr<'src>>,
  pub kw: Vec<(Ident<'src>, Expr<'src>)>,
}

impl<'src> Args<'src> {
  pub fn new() -> Self {
    Self {
      pos: Vec::new(),
      kw: Vec::new(),
    }
  }

  pub fn pos(&mut self, value: Expr<'src>) {
    self.pos.push(value);
  }

  pub fn kw(&mut self, name: Ident<'src>, value: Expr<'src>) {
    self.kw.push((name, value));
  }
}

impl<'src> Default for Args<'src> {
  fn default() -> Self {
    Self::new()
  }
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

pub fn stmt_pass<'src>(s: Range<usize>) -> Stmt<'src> {
  Stmt::new(s, StmtKind::Pass)
}

pub fn expr_binary<'src>(
  s: Range<usize>,
  op: BinaryOp,
  left: Expr<'src>,
  right: Expr<'src>,
) -> Expr<'src> {
  Expr::new(s, ExprKind::Binary(Box::new(Binary { op, left, right })))
}

pub fn expr_unary(s: Range<usize>, op: UnaryOp, right: Expr) -> Expr {
  Expr::new(s, ExprKind::Unary(Box::new(Unary { op, right })))
}

pub fn expr_call<'src>(s: Range<usize>, target: Expr<'src>, args: Args<'src>) -> Expr<'src> {
  Expr::new(s, ExprKind::Call(Box::new(Call { target, args })))
}

pub fn expr_index<'src>(s: Range<usize>, target: Expr<'src>, key: Expr<'src>) -> Expr<'src> {
  Expr::new(s, ExprKind::GetField(Box::new(GetField { target, key })))
}

pub fn expr_field<'src>(s: Range<usize>, target: Expr<'src>, key: Ident<'src>) -> Expr<'src> {
  expr_index(
    s,
    target,
    Expr::new(
      key.span,
      ExprKind::Literal(Box::new(Literal::String(key.into_inner()))),
    ),
  )
}

pub fn expr_array(s: Range<usize>, items: Vec<Expr>) -> Expr {
  Expr::new(s, ExprKind::Literal(Box::new(Literal::Array(items))))
}

pub fn ident_key(v: Ident) -> Expr {
  Expr::new(
    v.span,
    ExprKind::Literal(Box::new(Literal::String(v.into_inner().clone()))),
  )
}

pub fn expr_object<'src>(s: Range<usize>, items: Vec<(Expr<'src>, Expr<'src>)>) -> Expr<'src> {
  Expr::new(s, ExprKind::Literal(Box::new(Literal::Object(items))))
}

pub fn expr_get_var(name: Ident) -> Expr {
  Expr::new(name.span, ExprKind::GetVar(Box::new(GetVar { name })))
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

pub fn assign<'src>(
  target: Expr<'src>,
  kind: AssignKind,
  value: Expr<'src>,
) -> Result<Stmt<'src>, &'static str> {
  let span = target.span.start..value.span.end;
  match kind {
    AssignKind::Decl => {
      let name = match target.into_inner() {
        ExprKind::GetVar(target) => target.name,
        _ => return Err("@@invalid variable declaration"),
      };
      Ok(var_stmt(name, value))
    }
    AssignKind::Op(op) => {
      let assign = match target.into_inner() {
        ExprKind::GetVar(target) => ExprKind::SetVar(Box::new(SetVar {
          target: *target,
          op,
          value,
        })),
        ExprKind::GetField(target) => ExprKind::SetField(Box::new(SetField {
          target: *target,
          op,
          value,
        })),
        _ => return Err("@@invalid assignment target"),
      };
      Ok(expr_stmt(Expr::new(span, assign)))
    }
  }
}

pub fn loop_inf(s: Range<usize>, body: Vec<Stmt>) -> Stmt {
  Stmt::new(
    s,
    StmtKind::Loop(Box::new(Loop::Infinite(Infinite { body }))),
  )
}

pub fn loop_while<'src>(s: Range<usize>, cond: Expr<'src>, body: Vec<Stmt<'src>>) -> Stmt<'src> {
  Stmt::new(
    s,
    StmtKind::Loop(Box::new(Loop::While(While { cond, body }))),
  )
}

pub fn loop_for<'src>(
  s: Range<usize>,
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

  pub fn null<'src>(_: &str) -> Literal<'src> {
    Literal::Null
  }

  pub fn bool(lexeme: &str) -> Literal {
    let v = match lexeme {
      "true" => true,
      "false" => false,
      _ => unreachable!("bool is only ever `true` or `false`"),
    };
    Literal::Bool(v)
  }

  pub fn num(lexeme: &str) -> Option<Literal> {
    Some(Literal::Number(lexeme.parse().ok()?))
  }

  pub fn str(lexeme: &str) -> Literal {
    let lexeme = lexeme.strip_prefix('"').unwrap_or(lexeme);
    let lexeme = lexeme.strip_suffix('"').unwrap_or(lexeme);
    Literal::String(Cow::from(lexeme))
  }
}
