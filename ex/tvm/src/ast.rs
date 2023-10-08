#![allow(clippy::new_without_default)]

#[macro_use]
mod macros;

use crate::lex::Lexer;
use crate::lex::Span;
use crate::lex::Token;
use crate::lex::TokenKind;
use crate::Cow;

#[derive(Clone, Debug)]
pub struct SyntaxTree<'src> {
  pub top_level: Block<'src>,
}

decl! {
  Stmt<'src> {
    Let {
      name: Ident<'src>,
      ty: Option<Type<'src>>,
      init: Option<Expr<'src>>,
    },
    Loop {
      body: Block<'src>,
    },
    Fn {
      name: Ident<'src>,
      params: Vec<Param<'src>>,
      ret: Option<Type<'src>>,
      body: Block<'src>,
    },
    Record {
      name: Ident<'src>,
      fields: Vec<LetStmt<'src>>,
    },
    Expr {
      inner: Expr<'src>,
    },
  }
}

decl! {
  Type<'src> {
    Empty,
    Named {
      name: Ident<'src>,
    },
    Array {
      element: Type<'src>,
    },
    Set {
      value: Type<'src>,
    },
    Map {
      key: Type<'src>,
      value: Type<'src>,
    },
    Fn {
      params: Vec<Type<'src>>,
      ret: Type<'src>,
    },
  }
}

decl! {
  Expr<'src> {
    Never,
    Return {
      value: Expr<'src>,
    },
    Yield {
      value: Expr<'src>,
    },
    Break,
    Continue,
    Block {
      inner: Block<'src>,
    },
    If {
      branches: Vec<Branch<'src>>,
      tail: Option<Block<'src>>,
    },
    Binary {
      left: Expr<'src>,
      op: BinaryOp,
      right: Expr<'src>,
    },
    Unary {
      op: UnaryOp,
      right: Expr<'src>,
    },
    Literal {
      value: Literal<'src>,
    },
    Use {
      target: Place<'src>,
    },
    Assign {
      target: Place<'src>,
      value: Expr<'src>,
    },
    Call {
      target: Expr<'src>,
      args: Vec<Arg<'src>>,
    },
  }
}

#[derive(Clone, Debug)]
pub struct Param<'src> {
  pub name: Ident<'src>,
  pub ty: Type<'src>,
}

#[derive(Clone, Debug)]
pub struct Branch<'src> {
  pub cond: Expr<'src>,
  pub body: Block<'src>,
}

#[derive(Clone, Debug)]
pub struct Arg<'src> {
  pub key: Option<Ident<'src>>,
  pub value: Expr<'src>,
}

impl<'src> UseExpr<'src> {
  pub fn target(&self) -> &Place {
    &self.target
  }
}

#[derive(Clone, Debug)]
pub enum Place<'src> {
  Var {
    name: Ident<'src>,
  },
  Field {
    parent: Expr<'src>,
    name: Ident<'src>,
  },
  Index {
    parent: Expr<'src>,
    key: Expr<'src>,
  },
}

impl<'src> Place<'src> {
  pub fn is_var(&self) -> bool {
    matches!(self, Self::Var { .. })
  }

  pub fn into_var(self) -> Option<Ident<'src>> {
    if let Self::Var { name } = self {
      Some(name)
    } else {
      None
    }
  }
}

#[derive(Clone, Copy, Debug)]
pub enum BinaryOp {
  Add,
  Sub,
  Mul,
  Div,
  Rem,
  Pow,
  Eq,
  Ne,
  Gt,
  Lt,
  Ge,
  Le,
  And,
  Or,
  Opt,
}

macro_rules! binop {
  [+] => ($crate::ast::BinaryOp::Add);
  [-] => ($crate::ast::BinaryOp::Sub);
  [*] => ($crate::ast::BinaryOp::Mul);
  [/] => ($crate::ast::BinaryOp::Div);
  [%] => ($crate::ast::BinaryOp::Rem);
  [**] => ($crate::ast::BinaryOp::Pow);
  [==] => ($crate::ast::BinaryOp::Eq);
  [!=] => ($crate::ast::BinaryOp::Ne);
  [>] => ($crate::ast::BinaryOp::Gt);
  [<] => ($crate::ast::BinaryOp::Lt);
  [>=] => ($crate::ast::BinaryOp::Ge);
  [<=] => ($crate::ast::BinaryOp::Le);
  [&&] => ($crate::ast::BinaryOp::And);
  [||] => ($crate::ast::BinaryOp::Or);
  [??] => ($crate::ast::BinaryOp::Opt);
}

#[derive(Clone, Copy, Debug)]
pub enum UnaryOp {
  Minus,
  Not,
  Opt,
}

macro_rules! unop {
  [-] => ($crate::ast::UnaryOp::Minus);
  [!] => ($crate::ast::UnaryOp::Not);
  [?] => ($crate::ast::UnaryOp::Opt);
}

#[derive(Clone, Debug)]
pub enum Literal<'src> {
  None,
  Int(i64),
  Float(f64),
  Bool(bool),
  String(Cow<'src, str>),
  Array(Vec<Expr<'src>>),
  // TODO: during type check, empty `map` can coerce to `set`
  Set(Vec<Expr<'src>>),
  Map(Vec<(Expr<'src>, Expr<'src>)>),
}

macro_rules! lit {
  (none) => {
    $crate::ast::Literal::None
  };
  (int, $v:expr) => {
    $crate::ast::Literal::Int(($v).into())
  };
  (float, $v:expr) => {
    $crate::ast::Literal::Float(($v).into())
  };
  (bool, $v:expr) => {
    $crate::ast::Literal::Bool(($v).into())
  };
  (str, $v:expr) => {
    $crate::ast::Literal::String(($v).into())
  };
  (array, $v:expr) => {
    $crate::ast::Literal::Array(($v).into())
  };
  (set, $v:expr) => {
    $crate::ast::Literal::Set(($v).into())
  };
  (map, $v:expr) => {
    $crate::ast::Literal::Map(($v).into())
  };
}

impl Default for Literal<'_> {
  fn default() -> Self {
    Self::Int(0)
  }
}

#[derive(Clone, Debug)]
pub struct Block<'src> {
  pub span: Span,
  pub body: Vec<Stmt<'src>>,
  pub tail: Option<Expr<'src>>,
}

impl<'src> From<Block<'src>> for Stmt<'src> {
  fn from(block: Block<'src>) -> Self {
    Stmt::make_expr(block.span, Expr::make_block(block.span, block))
  }
}

impl<'src> From<Expr<'src>> for Stmt<'src> {
  fn from(value: Expr<'src>) -> Self {
    Stmt::make_expr(value.span, value)
  }
}

#[derive(Clone, Debug)]
pub struct Ident<'src> {
  pub span: Span,
  pub lexeme: &'src str,
}

impl<'src> Ident<'src> {
  pub fn from_token(l: &Lexer<'src>, t: &Token) -> Self {
    assert!(matches!(t.kind, TokenKind::Ident));
    Self {
      span: t.span,
      lexeme: l.lexeme(t),
    }
  }
}

trait WrapBox {
  type Boxed;

  fn wrap_box(self) -> Self::Boxed;
}

trait UnwrapBox {
  type Unboxed;

  fn unwrap_box(self) -> Self::Unboxed;
}
