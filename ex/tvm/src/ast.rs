#![allow(clippy::new_without_default)]

use crate::lex::Span;
use crate::Cow;

macro_rules! decl {
  (
    $name:ident<$lifetime:lifetime> {
      $(
        $variant:ident $({
          $($field:ident : $ty:ty),* $(,)?
        })?
      ),* $(,)?
    }
  ) => {
    paste::paste! {
      #[derive(Clone, Debug)]
      pub struct $name<$lifetime> {
        pub kind: [<$name Kind>]<$lifetime>,
        pub span: Span,
      }

      impl<$lifetime> $name<$lifetime> {
        $(
          pub fn [<make_ $variant:snake>](
            span: impl Into<crate::lex::Span>,
            $($($field : $ty),*)?
          ) -> Self {
            Self {
              kind: [<$name Kind>]::$variant(
                Box::new(
                  [<$variant $name>]::new(
                    $($($field),*)?
                  )
                )
              ),
              span: span.into(),
            }
          }
        )*
      }

      #[derive(Clone, Debug)]
      pub enum [<$name Kind>]<$lifetime> {
        $(
          $variant(Box<[<$variant $name>]<$lifetime>>)
        ),*
      }
    }

    $(
      decl!(@variant_struct $variant $name $lifetime $({ $($field : $ty),* })?);
    )*
  };

  (@variant_struct
    $variant:ident
    $name:ident
    $lifetime:lifetime
    { $($field:ident : $ty:ty),* }
  ) => {
    paste::paste! {
      #[derive(Clone, Debug)]
      pub struct [<$variant $name>]<$lifetime> {
        _lifetime: std::marker::PhantomData<& $lifetime ()>,
        $(pub $field : $ty),*
      }

      impl<$lifetime> [<$variant $name>]<$lifetime> {
        pub fn new($($field : $ty),*) -> Self {
          Self {
            _lifetime: std::marker::PhantomData,
            $($field),*
          }
        }
      }
    }
  };

  (@variant_struct
    $variant:ident
    $name:ident
    $lifetime:lifetime
  ) => {
    paste::paste! {
      #[derive(Clone, Debug)]
      pub struct [<$variant $name>]<$lifetime>(std::marker::PhantomData<& $lifetime ()>);

      impl<$lifetime> [<$variant $name>]<$lifetime> {
        pub fn new() -> Self {
          Self(std::marker::PhantomData)
        }
      }
    }
  };
}

#[derive(Clone, Debug)]
pub struct SyntaxTree<'a> {
  pub top_level: Vec<Stmt<'a>>,
}

decl! {
  Stmt<'a> {
    Var {
      name: Ident<'a>,
      ty: Type<'a>,
      init: Expr<'a>,
    },
    Loop,
    While {
      cond: Expr<'a>,
    },
    ForIter {
      item: Ident<'a>,
      ty: Type<'a>,
      iter: Expr<'a>,
    },
    ForRange {
      item: Ident<'a>,
      ty: Type<'a>,
      start: Expr<'a>,
      end: Expr<'a>,
      inclusive: bool,
    },
    Fn {
      vis: Vis,
      name: Ident<'a>,
      sig: FnSig<'a>,
      body: Block<'a>,
    },
    Class {
      vis: Vis,
      name: Ident<'a>,
      type_params: Vec<Ident<'a>>,
      super_class: Type<'a>,
      bounds: Vec<Bound<'a>>,
      members: ClassMembers<'a>,
    },
    Inter {
      vis: Vis,
      name: Ident<'a>,
      type_params: Vec<Ident<'a>>,
      super_inter: Type<'a>,
      bounds: Vec<Bound<'a>>,
      members: InterMembers<'a>,
    },
    Impl {
      type_params: Vec<Ident<'a>>,
      inter: Option<Type<'a>>,
      target: Type<'a>,
      bounds: Vec<Bound<'a>>,
      members: InterMembers<'a>,
    },
    TypeAlias {
      vis: Vis,
      name: Ident<'a>,
      type_params: Vec<Ident<'a>>,
      target: Type<'a>,
    },
    Expr {
      inner: Expr<'a>,
    },
  }
}

#[derive(Clone, Debug)]
pub struct ClassMembers<'a> {
  pub fields: Vec<VarStmt<'a>>,
  pub methods: Vec<FnStmt<'a>>,
}

#[derive(Clone, Debug)]
pub struct InterMembers<'a> {
  pub methods: Vec<FnStmt<'a>>,
}

#[derive(Clone, Copy, Debug)]
pub enum Vis {
  Pub,
  Priv,
}

decl! {
  Type<'a> {
    Named {
      name: Ident<'a>,
      opt: bool,
    },
    Generic {
      name: Ident<'a>,
      type_params: Vec<Ident<'a>>,
      opt: bool,
    },
    Tuple {
      items: Vec<Type<'a>>,
    },
    Array {
      element: Type<'a>,
      opt: bool,
    },
    Fn {
      params: Vec<Type<'a>>,
      ret: Option<Type<'a>>,
      body: Block<'a>,
    },
    Unknown,
  }
}

decl! {
  Expr<'a> {
    Return {
      value: Option<Expr<'a>>,
    },
    Yield {
      value: Option<Expr<'a>>,
    },
    Break {
      label: Option<Label<'a>>,
    },
    Continue {
      label: Option<Label<'a>>,
    },
    Block {
      inner: Block<'a>,
    },
    If {
      branches: Vec<(Expr<'a>, Block<'a>)>,
      tail: Option<Block<'a>>,
    },
    Fn {
      name: Option<Ident<'a>>,
      sig: FnSig<'a>,
    },
    Binary {
      left: Expr<'a>,
      op: BinaryOp,
      right: Expr<'a>,
    },
    Unary {
      op: UnaryOp,
      right: Expr<'a>,
    },
    Literal {
      inner: Literal<'a>,
    },
    GetVar {
      name: Ident<'a>,
    },
    SetVar {
      name: Ident<'a>,
      value: Expr<'a>,
    },
    GetField {
      target: Expr<'a>,
      name: Ident<'a>,
    },
    SetField {
      target: Expr<'a>,
      name: Ident<'a>,
      value: Expr<'a>,
    },
    GetIndex {
      target: Expr<'a>,
      key: Expr<'a>,
    },
    SetIndex {
      target: Expr<'a>,
      key: Expr<'a>,
      value: Expr<'a>,
    },
    GetSelf,
    GetSuper,
    Call {
      target: Expr<'a>,
      args: Vec<Arg<'a>>,
    },
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

#[derive(Clone, Copy, Debug)]
pub enum UnaryOp {
  Minus,
  Not,
  Opt,
}

#[derive(Clone, Debug)]
pub enum Literal<'a> {
  Int(i64),
  Float(f64),
  Bool(bool),
  String(Cow<'a, str>),
  Tuple(Vec<Expr<'a>>),
  Array(Vec<Expr<'a>>),
  Map(Vec<(Expr<'a>, Expr<'a>)>),
}

#[derive(Clone, Debug)]
pub struct FnSig<'a> {
  pub type_params: Vec<Ident<'a>>,
  pub params: Vec<(Ident<'a>, Type<'a>)>,
  pub ret: Option<Type<'a>>,
  pub bounds: Vec<Bound<'a>>,
}

#[derive(Clone, Debug)]
pub struct Bound<'a> {
  pub left: Type<'a>,
  pub right: Vec<Type<'a>>,
}

#[derive(Clone, Debug)]
pub struct Block<'a> {
  pub body: Vec<Stmt<'a>>,
  pub tail: Option<Expr<'a>>,
}

#[derive(Clone, Debug)]
pub enum Arg<'a> {
  Labelled { value: Expr<'a>, label: Ident<'a> },
  Plain { value: Expr<'a> },
}

#[derive(Clone, Debug)]
pub struct Ident<'src> {
  pub span: Span,
  pub lexeme: &'src str,
}

#[derive(Clone, Debug)]
pub struct Label<'src> {
  pub span: Span,
  pub lexeme: &'src str,
}
