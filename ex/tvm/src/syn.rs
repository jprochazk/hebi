use crate::ast::*;
use crate::error::Error;
use crate::error::Result;
use crate::lex::Lexer;
use crate::lex::Span;
use crate::lex::Token;
use crate::lex::TokenKind;
use crate::lex::EOF;
use std::cell::Cell;

// TODO: do not emit errors in the middle of a broken syntax node

pub struct Parser<'src> {
  lex: Lexer<'src>,
  prev: Token,
  curr: Token,
  errors: Vec<Error>,
}

struct Meta {
  errored: Cell<bool>,
  span_start: u32,
}

impl<'src> Parser<'src> {
  #[inline]
  pub fn new(src: &'src str) -> Parser<'src> {
    Parser {
      lex: Lexer::new(src),
      prev: EOF,
      curr: EOF,
      errors: Vec::new(),
    }
  }

  pub fn parse(mut self) -> Result<SyntaxTree<'src>, Vec<Error>> {
    let top_level = top_level(&mut self);

    if !self.errors.is_empty() {
      return Err(self.errors);
    }

    Ok(SyntaxTree { top_level })
  }

  #[inline]
  fn meta(&self) -> Meta {
    Meta {
      errored: Cell::new(false),
      span_start: self.curr.span.start,
    }
  }

  #[inline]
  fn finish(&self, m: Meta) -> Span {
    Span {
      start: m.span_start,
      end: self.prev.span.end,
    }
  }

  #[inline]
  fn error(&mut self, m: &Meta, e: Error) {
    if !m.errored.get() {
      m.errored.set(true);
      self.errors.push(e);
    }
  }

  #[inline]
  fn end(&self) -> bool {
    self.curr.is(t![EOF])
  }

  #[inline]
  fn kind(&self) -> TokenKind {
    self.curr.kind
  }

  #[inline]
  fn at(&mut self, kind: TokenKind) -> bool {
    self.curr.is(kind)
  }

  #[inline]
  fn eat(&mut self, kind: TokenKind) -> bool {
    let found = self.at(kind);
    if found {
      self.advance();
    }
    found
  }

  #[inline]
  fn must(&mut self, m: &Meta, kind: TokenKind) -> bool {
    if self.eat(kind) {
      true
    } else {
      self.error(m, err!(@self.curr.span, ExpectedToken, kind));
      false
    }
  }

  #[inline]
  fn must2(&mut self, kind: TokenKind) -> bool {
    self.must(&self.meta(), kind)
  }

  #[inline]
  fn advance(&mut self) -> Token {
    self.prev = self.curr;
    self.curr = loop {
      match self.lex.bump() {
        Ok(token) => break token,
        Err(e) => self.errors.push(e),
      }
    };
    self.prev
  }

  #[inline]
  fn lexeme(&self, token: &Token) -> &'src str {
    self.lex.lexeme(token)
  }
}

fn top_level<'src>(p: &mut Parser<'src>) -> Block<'src> {
  let m = p.meta();

  let mut body = vec![];
  if !p.end() {
    body.push(stmt(p));
  }
  while !p.end() {
    // we try to parse the stmt even if there is no semicolon
    p.must2(t![;]);
    body.push(stmt(p));
  }
  // the tail of a block is the final stmt iff:
  // - it is an expression
  // - it is not terminated by a semicolon (`;`)
  let tail = if !p.eat(t![;]) && body.last().is_some_and(Stmt::is_expr) {
    Some(body.pop().unwrap().into_expr().unwrap().inner)
  } else {
    None
  };

  Block {
    span: p.finish(m),
    body,
    tail,
  }
}

fn stmt<'src>(p: &mut Parser<'src>) -> Stmt<'src> {
  match p.kind() {
    t![fn] => fn_(p),
    t![loop] => loop_(p),
    t![let] => let_(p),
    t![break] => break_(p).into(),
    t![continue] => continue_(p).into(),
    t![return] => return_(p).into(),
    t![if] => if_(p).into(),
    t![yield] => yield_(p).into(),
    t!["{"] => block(p).into(),
    _ => expr(&p.meta(), p).into(),
  }
}

fn fn_<'src>(p: &mut Parser<'src>) -> Stmt<'src> {
  let m = p.meta();

  assert!(p.eat(t![fn]));
  let name = ident(&m, p);
  let params = params(&m, p);
  let ret = p.eat(t![->]).then(|| type_(&m, p));
  let body = block(p);
  let span = p.finish(m);
  Stmt::make_fn(span, name, params, ret, body)
}

fn param<'src>(m: &Meta, p: &mut Parser<'src>) -> Param<'src> {
  Param {
    name: ident(m, p),
    ty: type_(m, p),
  }
}

fn params<'src>(m: &Meta, p: &mut Parser<'src>) -> Vec<Param<'src>> {
  paren_list(m, p, param)
}

fn paren_list<'src, F, T>(m: &Meta, p: &mut Parser<'src>, f: F) -> Vec<T>
where
  F: Fn(&Meta, &mut Parser<'src>) -> T,
{
  p.must(m, t!["("]);
  let mut out = vec![];
  if !p.end() && !p.at(t![")"]) {
    out.push(f(m, p));
    while !p.end() && p.eat(t![,]) && !p.at(t![")"]) {
      out.push(f(m, p));
    }
  }
  p.must(m, t![")"]);
  out
}

fn loop_<'src>(p: &mut Parser<'src>) -> Stmt<'src> {
  let m = p.meta();

  assert!(p.eat(t![loop]));
  let body = block(p);
  let span = p.finish(m);
  Stmt::make_loop(span, body)
}

fn let_<'src>(p: &mut Parser<'src>) -> Stmt<'src> {
  let m = p.meta();

  let name = ident(&m, p);
  let ty = p.eat(t![:]).then(|| type_(&m, p));
  let init = p.eat(t![=]).then(|| expr(&m, p));

  let span = p.finish(m);
  Stmt::make_let(span, name, ty, init)
}

fn break_<'src>(p: &mut Parser<'src>) -> Expr<'src> {
  assert!(p.eat(t![break]));
  let span = p.prev.span;
  Expr::make_break(span)
}

fn continue_<'src>(p: &mut Parser<'src>) -> Expr<'src> {
  assert!(p.eat(t![continue]));
  let span = p.prev.span;
  Expr::make_continue(span)
}

fn return_<'src>(p: &mut Parser<'src>) -> Expr<'src> {
  let m = p.meta();

  assert!(p.eat(t![return]));
  let value = expr(&m, p);
  let span = p.finish(m);
  Expr::make_return(span, value)
}

fn yield_<'src>(p: &mut Parser<'src>) -> Expr<'src> {
  let m = p.meta();

  assert!(p.eat(t![yield]));
  let value = expr(&m, p);
  let span = p.finish(m);
  Expr::make_yield(span, value)
}

fn if_<'src>(p: &mut Parser<'src>) -> Expr<'src> {
  let m = p.meta();

  assert!(p.eat(t![if]));
  let mut branches = vec![branch(&m, p)];
  let mut tail = None;
  while !p.end() && p.eat(t![else]) {
    if p.eat(t![if]) {
      branches.push(branch(&m, p));
    } else {
      tail = Some(block(p));
    }
  }

  let span = p.finish(m);
  Expr::make_if(span, branches, tail)
}

fn branch<'src>(m: &Meta, p: &mut Parser<'src>) -> Branch<'src> {
  Branch {
    cond: expr(m, p),
    body: block(p),
  }
}

// TODO: opt
fn type_<'src>(m: &Meta, p: &mut Parser<'src>) -> Type<'src> {
  match p.kind() {
    t![_] => {
      p.advance();
      Type::make_empty(p.prev.span)
    }
    t![ident] => {
      let ident = ident(m, p);
      Type::make_named(ident.span, ident)
    }
    t!["["] => {
      let start = p.curr.span.start;
      p.advance();
      let element = type_(m, p);
      let end = p.prev.span.end;
      p.must(m, t!["]"]);
      Type::make_array(start..end, element)
    }
    t!["{"] => {
      let start = p.curr.span.start;
      p.advance();
      let key = type_(m, p);
      if p.eat(t![->]) {
        let value = type_(m, p);
        p.must(m, t!["}"]);
        let end = p.prev.span.end;
        Type::make_map(start..end, key, value)
      } else {
        p.must(m, t!["}"]);
        let end = p.prev.span.end;
        Type::make_set(start..end, key)
      }
    }
    t!["("] => {
      let start = p.curr.span.start;

      let end = p.prev.span.end;
      let params = paren_list(m, p, type_);
      p.must(m, t![")"]);
      p.must(m, t![->]);
      let ret = type_(m, p);
      Type::make_fn(start..end, params, ret)
    }
    _ => {
      p.error(m, err!(@p.curr.span, UnexpectedToken));
      Type::make_empty(Span::empty())
    }
  }
}

fn ident<'src>(m: &Meta, p: &mut Parser<'src>) -> Ident<'src> {
  if p.must(m, t![ident]) {
    Ident {
      span: p.prev.span,
      lexeme: p.lex.lexeme(&p.prev),
    }
  } else {
    Ident {
      span: Span::empty(),
      lexeme: "",
    }
  }
}

fn block<'src>(p: &mut Parser<'src>) -> Block<'src> {
  let m = p.meta();

  assert!(p.eat(t!["{"]));
  let mut body = vec![];
  if !p.end() && !p.at(t!["}"]) {
    body.push(stmt(p));
  }
  while !p.end() && !p.at(t!["}"]) {
    // we try to parse the stmt even if there is no semicolon
    p.must2(t![;]);
    body.push(stmt(p));
  }
  // the tail of a block is the final stmt if:
  // - it is an expression
  // - it is not terminated by a semicolon (`;`)
  let tail = if !p.eat(t![;]) && body.last().is_some_and(Stmt::is_expr) {
    Some(body.pop().unwrap().into_expr().unwrap().inner)
  } else {
    None
  };
  p.must2(t!["}"]);

  let span = p.finish(m);
  Block { span, body, tail }
}

fn expr<'src>(m: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
  expr::or(m, p)
}

mod expr {
  use super::*;

  fn binary<'src>(lhs: Expr<'src>, op: BinaryOp, rhs: Expr<'src>) -> Expr<'src> {
    Expr::make_binary(lhs.span.to(rhs.span), lhs, op, rhs)
  }

  pub(super) fn or<'src>(m: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
    let mut lhs = and(m, p);
    while !p.end() && p.eat(t![||]) {
      let rhs = and(m, p);
      lhs = binary(lhs, binop![||], rhs);
    }
    lhs
  }

  fn and<'src>(m: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
    let mut lhs = eq(m, p);
    while !p.end() && p.eat(t![&&]) {
      let rhs = eq(m, p);
      lhs = binary(lhs, binop![&&], rhs);
    }
    lhs
  }

  fn eq<'src>(m: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
    let mut lhs = cmp(m, p);
    while !p.end() {
      let op = match p.kind() {
        t![==] => binop![==],
        t![!=] => binop![!=],
        _ => break,
      };
      let rhs = cmp(m, p);
      lhs = binary(lhs, op, rhs);
    }
    lhs
  }

  fn cmp<'src>(m: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
    let mut lhs = add(m, p);
    while !p.end() {
      let op = match p.kind() {
        t![>] => binop![>],
        t![>=] => binop![>=],
        t![<] => binop![<],
        t![<=] => binop![<=],
        _ => break,
      };
      let rhs = add(m, p);
      lhs = binary(lhs, op, rhs);
    }
    lhs
  }

  fn add<'src>(m: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
    let mut lhs = mul(m, p);
    while !p.end() {
      let op = match p.kind() {
        t![+] => binop![+],
        t![-] => binop![-],
        _ => break,
      };
      let rhs = mul(m, p);
      lhs = binary(lhs, op, rhs);
    }
    lhs
  }

  fn mul<'src>(m: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
    let mut lhs = pow(m, p);
    while !p.end() {
      let op = match p.kind() {
        t![*] => binop![*],
        t![/] => binop![/],
        t![%] => binop![%],
        _ => break,
      };
      let rhs = pow(m, p);
      lhs = binary(lhs, op, rhs);
    }
    lhs
  }

  fn pow<'src>(m: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
    let mut lhs = pre(m, p);
    while !p.end() && p.eat(t![**]) {
      let rhs = pre(m, p);
      lhs = binary(lhs, binop![**], rhs);
    }
    lhs
  }

  fn pre<'src>(m: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
    let op = match p.kind() {
      t![-] => unop![-],
      t![!] => unop![!],
      t![?] => unop![?],
      _ => return post(m, p),
    };
    let tok = p.advance();
    let rhs = pre(m, p);
    Expr::make_unary(tok.span.to(rhs.span), op, rhs)
  }

  fn post<'src>(m: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
    let mut expr = primary(m, p);
    while !p.end() {
      match p.kind() {
        t!["("] => expr = call(m, p, expr),
        t!["["] => expr = index(m, p, expr),
        t![.] => expr = field(m, p, expr),
        _ => break,
      };
    }
    expr
  }

  fn call<'src>(m: &Meta, p: &mut Parser<'src>, target: Expr<'src>) -> Expr<'src> {
    let f = p.meta();

    assert!(p.eat(t!["("]));
    let mut args = vec![];
    if !p.end() && !p.at(t![")"]) {
      args.push(arg(m, p))
    }
    while !p.end() && p.eat(t![,]) && !p.at(t![")"]) {
      args.push(arg(m, p))
    }
    p.must(m, t![")"]);

    let span = p.finish(f);
    Expr::make_call(span, target, args)
  }

  fn arg<'src>(m: &Meta, p: &mut Parser<'src>) -> Arg<'src> {
    let value = expr(m, p);
    let (key, value) = if value.as_use().is_some_and(|v| v.target.is_var()) {
      let key = Some(value.into_use().unwrap().target.into_var().unwrap());
      let value = expr(m, p);
      (key, value)
    } else {
      (None, value)
    };
    Arg { key, value }
  }

  fn index<'src>(m: &Meta, p: &mut Parser<'src>, parent: Expr<'src>) -> Expr<'src> {
    let f = p.meta();

    assert!(p.eat(t!["["]));
    let key = expr(m, p);
    p.must(m, t!["]"]);

    let span = p.finish(f);
    Expr::make_use(span, Place::Index { parent, key })
  }

  fn field<'src>(m: &Meta, p: &mut Parser<'src>, parent: Expr<'src>) -> Expr<'src> {
    let f = p.meta();

    assert!(p.eat(t![.]));
    let name = ident(m, p);

    let span = p.finish(f);
    Expr::make_use(span, Place::Field { parent, name })
  }

  fn primary<'src>(m: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
    match p.kind() {
      t![int] => int(m, p),
      t![float] => float(m, p),
      t![bool] => bool(m, p),
      t![none] => none(m, p),
      t![str] => str(m, p),
      t![if] => if_(p),
      t![do] => do_(m, p),
      t!["["] => array(m, p),
      t!["{"] => set_or_map(m, p),
      t![ident] => use_(m, p),
      _ => {
        let span = p.curr.span;
        p.error(m, err!(@span, UnexpectedToken));
        Expr::make_never(span)
      }
    }
  }

  fn int<'src>(m: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
    assert!(p.eat(t![float]));
    let span = p.prev.span;
    let v = p.lexeme(&p.prev).parse::<i64>().unwrap_or_else(|_| {
      p.error(m, err!(@span, InvalidInt));
      0i64
    });
    Expr::make_literal(span, lit!(int, v))
  }

  fn float<'src>(m: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
    assert!(p.eat(t![float]));
    let span = p.prev.span;
    let v = p.lexeme(&p.prev).parse::<f64>().unwrap_or_else(|_| {
      p.error(m, err!(@span, InvalidFloat));
      0f64
    });
    Expr::make_literal(span, lit!(float, v))
  }

  fn bool<'src>(_: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
    assert!(p.eat(t![bool]));
    let v = match p.lexeme(&p.prev) {
      "true" => true,
      "false" => false,
      _ => unreachable!(),
    };
    Expr::make_literal(p.prev.span, lit!(bool, v))
  }

  fn none<'src>(_: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
    assert!(p.eat(t![none]));
    Expr::make_literal(p.prev.span, lit!(none))
  }

  fn str<'src>(_: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
    assert!(p.eat(t![str]));
    // TODO: unescape
    // TODO: fmt
    Expr::make_literal(p.prev.span, lit!(str, p.lexeme(&p.prev)))
  }

  fn do_<'src>(_: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
    assert!(p.eat(t![do]));
    let body = block(p);
    Expr::make_block(body.span, body)
  }

  fn array<'src>(m: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
    let f = p.meta();

    assert!(p.eat(t!["["]));
    let mut elems = vec![];
    if !p.end() && !p.at(t!["]"]) {
      elems.push(expr(m, p));
    }
    while !p.end() && p.eat(t![,]) && !p.at(t!["]"]) {
      elems.push(expr(m, p));
    }
    p.must(m, t!["]"]);

    Expr::make_literal(p.finish(f), lit!(array, elems))
  }

  fn set_or_map<'src>(m: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
    let f = p.meta();

    assert!(p.eat(t!["{"]));
    if p.end() {
      p.must(m, t!["}"]);
      return Expr::make_literal(p.finish(f), lit!(map, vec![]));
    }

    if p.eat(t!["}"]) {
      return Expr::make_literal(p.finish(f), lit!(map, vec![]));
    }

    let first_key = expr(m, p);
    if p.eat(t![:]) {
      let first_value = expr(m, p);
      map(m, p, f, first_key, first_value)
    } else {
      set(m, p, f, first_key)
    }
  }

  fn map<'src>(
    m: &Meta,
    p: &mut Parser<'src>,
    f: Meta,
    first_key: Expr<'src>,
    first_value: Expr<'src>,
  ) -> Expr<'src> {
    let mut items = vec![(first_key, first_value)];
    while !p.end() && p.eat(t![,]) && !p.at(t!["}"]) {
      let key = expr(m, p);
      p.must(m, t![:]);
      let value = expr(m, p);
      items.push((key, value));
    }
    p.must(m, t!["}"]);
    Expr::make_literal(p.finish(f), lit!(map, items))
  }

  fn set<'src>(m: &Meta, p: &mut Parser<'src>, f: Meta, first_key: Expr<'src>) -> Expr<'src> {
    let mut items = vec![first_key];
    while !p.end() && p.eat(t![,]) && !p.at(t!["}"]) {
      let key = expr(m, p);
      items.push(key);
    }
    p.must(m, t!["}"]);
    Expr::make_literal(p.finish(f), lit!(set, items))
  }

  fn use_<'src>(_: &Meta, p: &mut Parser<'src>) -> Expr<'src> {
    assert!(p.eat(t![ident]));
    let name = Ident::from_token(&p.lex, &p.prev);
    Expr::make_use(p.prev.span, Place::Var { name })
  }
}
