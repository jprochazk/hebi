macro_rules! binary {
  ($left:ident $op:tt $right:ident) => {{
    let left = $left;
    let op = binary!(__operator $op);
    let right = $right;
    $crate::ast::expr_binary(left.span.start..right.span.end, op, left, right)
  }};

  (__operator (+)) => {
    $crate::ast::BinaryOp::Add
  };
  (__operator (-)) => {
    $crate::ast::BinaryOp::Sub
  };
  (__operator (/)) => {
    $crate::ast::BinaryOp::Div
  };
  (__operator (**)) => {
    $crate::ast::BinaryOp::Pow
  };
  (__operator (*)) => {
    $crate::ast::BinaryOp::Mul
  };
  (__operator (%)) => {
    $crate::ast::BinaryOp::Rem
  };
  (__operator (==)) => {
    $crate::ast::BinaryOp::Eq
  };
  (__operator (!=)) => {
    $crate::ast::BinaryOp::Neq
  };
  (__operator (>)) => {
    $crate::ast::BinaryOp::More
  };
  (__operator (>=)) => {
    $crate::ast::BinaryOp::MoreEq
  };
  (__operator (<)) => {
    $crate::ast::BinaryOp::Less
  };
  (__operator (<=)) => {
    $crate::ast::BinaryOp::LessEq
  };
  (__operator (&&)) => {
    $crate::ast::BinaryOp::And
  };
  (__operator (||)) => {
    $crate::ast::BinaryOp::Or
  };
  (__operator (??)) => {
    $crate::ast::BinaryOp::Maybe
  };
}

macro_rules! unary {
  ($start:ident $op:tt $right:ident) => {{
    let op = unary!(__operator $op);
    let start = $start;
    let right = $right;
    $crate::ast::expr_unary(start..right.span.end, op, right)
  }};
  (__operator (-)) => {
    $crate::ast::UnaryOp::Minus
  };
  (__operator (!)) => {
    $crate::ast::UnaryOp::Not
  };
}

macro_rules! literal {
  ($state:ident @ $pos:ident , $ty:ident) => {{
    let state = $state;
    let pos = $pos;
    let tok = state.borrow_mut().lexer.get(pos).unwrap();
    let lexeme = state.borrow().lexer.lexeme(tok);
    $crate::ast::Expr::new(
      tok.span,
      $crate::ast::ExprKind::Literal(::std::boxed::Box::new($crate::ast::lit::$ty(lexeme))),
    )
  }};

  ($state:ident @ $pos:ident , $ty:ident ? ) => {{
    let state = $state;
    let pos = $pos;
    let tok = state.borrow_mut().lexer.get(pos).unwrap();
    let lexeme = state.borrow().lexer.lexeme(tok);
    let Some(inner) = $crate::ast::lit::$ty(lexeme) else { return Err(stringify!($ty)); };
    Ok($crate::ast::Expr::new(
      tok.span,
      $crate::ast::ExprKind::Literal(Box::new(inner)),
    ))
  }};
}

macro_rules! s {
  ($state:ident) => {
    $state.borrow_mut()
  };
}

macro_rules! take {
  ($state:ident . $what:ident) => {
    ::std::mem::take(&mut $state.borrow_mut().temp.$what)
  };
}
