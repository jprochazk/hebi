use std::fmt::Display;

use bumpalo::collections::Vec;
use bumpalo::vec;
use TokenKind::*;

use super::ast::*;
use super::lex::*;

macro_rules! err {
  ($self:ident, @$span:expr, $($args:tt)*) => {
    $self.error(
      format_args!($($args)*),
      $span,
    )
  };
  ($self:ident, $($args:tt)*) => {
    $self.error(
      format_args!($($args)*),
      $self.current().span,
    )
  };
}

pub type Arena = bumpalo::Bump;

pub struct Parser<'arena, 'src> {
  arena: &'arena Arena,
  lex: Lexer<'src>,
}

pub type Result<T, E = String> = std::result::Result<T, E>;

impl<'arena, 'src> Parser<'arena, 'src> {
  pub fn new(arena: &'arena Arena, lex: Lexer<'src>) -> Self {
    Self { arena, lex }
  }

  pub fn parse(mut self) -> Result<Module<'arena, 'src>> {
    self.program()
  }

  fn program(&mut self) -> Result<Module<'arena, 'src>> {
    let mut list = vec![in self.arena];
    while !self.end() {
      list.push(self.stmt()?);
    }
    Ok(list.into_bump_slice())
  }

  fn error(&self, message: impl Display, span: impl Into<Span>) -> String {
    use std::fmt::Write;

    let span: Span = span.into();
    let src = self.lex.src();
    let mut out = String::new();

    // empty span
    if span.start == span.end {
      writeln!(&mut out, "syntax error: {message} at {span}").unwrap();
      return out;
    }

    writeln!(&mut out, "{message} at {span}:").unwrap();
    let line_start = src[..span.start()].rfind('\n').map(|v| v + 1).unwrap_or(0);
    let line_num = 1 + src[..line_start].lines().count();
    let line_num_width = num_digits(line_num);
    let line_end = src[span.start()..]
      .find('\n')
      .map(|v| v + span.start())
      .unwrap_or(src.len());
    writeln!(&mut out, "{line_num} |  {}", &src[line_start..line_end]).unwrap();
    let cursor_pos = span.start() - line_start;
    let cursor_len = if span.end() > line_end {
      line_end - span.start()
    } else {
      span.end() - span.start()
    };
    writeln!(
      &mut out,
      "{:lw$} |  {:w$}{:^<l$}",
      "",
      "",
      "^",
      lw = line_num_width,
      w = cursor_pos,
      l = cursor_len
    )
    .unwrap();

    out
  }

  fn alloc<T>(&self, v: T) -> &'arena T {
    self.arena.alloc(v)
  }

  fn end(&self) -> bool {
    self.lex.current().is(TokEof)
  }

  fn current(&self) -> &Token {
    self.lex.current()
  }

  fn previous(&self) -> &Token {
    self.lex.previous()
  }

  fn at(&mut self, kind: TokenKind) -> bool {
    self.lex.current().is(kind)
  }

  fn eat(&mut self, kind: TokenKind) -> Result<bool> {
    let found = self.at(kind);
    if found {
      self.bump()?;
    }
    Ok(found)
  }

  fn expect(&mut self, kind: TokenKind) -> Result<()> {
    if !self.eat(kind)? {
      return Err(err!(self, "expected `{}`", kind.name()));
    }
    Ok(())
  }

  fn bump(&mut self) -> Result<&Token> {
    self.lex.bump();
    if self.at(TokError) {
      return Err(err!(
        self,
        "invalid token `{}`",
        self.lex.lexeme(self.current()),
      ));
    }
    Ok(self.previous())
  }

  fn ident(&mut self) -> Result<Ident<'src>> {
    self.expect(TokIdent)?;
    Ok(Ident {
      span: self.previous().span,
      lexeme: self.lex.lexeme(self.previous()),
    })
  }

  fn param_list(&mut self) -> Result<Vec<'arena, Ident<'src>>> {
    self.expect(BrkParenL)?;
    let mut list = vec![in self.arena];
    if !self.end() && !self.at(BrkParenR) {
      list.push(self.ident()?);
      while !self.end() && self.eat(TokComma)? && !self.at(BrkParenR) {
        list.push(self.ident()?);
      }
    }
    self.expect(BrkParenR)?;
    Ok(list)
  }

  fn block(&mut self) -> Result<Block<'arena, 'src>> {
    self.expect(BrkCurlyL)?;
    let mut body = vec![in self.arena];
    if !self.end() && !self.at(BrkCurlyR) {
      body.push(self.stmt()?);
      while !self.end() && !self.at(BrkCurlyR) {
        body.push(self.stmt()?);
      }
    }
    self.expect(BrkCurlyR)?;

    let last = match body.last() {
      Some(Stmt {
        kind: StmtKind::Expr(expr),
        ..
      }) => {
        let last = **expr;
        body.pop();
        Some(last)
      }
      Some(..) | None => None,
    };

    Ok(Block {
      body: body.into_bump_slice(),
      last,
    })
  }
}

mod stmt {
  use super::*;

  macro_rules! mk {
    ($self:ident, $kind:ident @ $span:expr) => {
      Stmt::new(StmtKind::$kind, Span::from($span))
    };
    ($self:ident, $kind:ident ( $inner:expr ) @ $span:expr) => {
      Stmt::new(StmtKind::$kind($self.alloc($inner)), Span::from($span))
    };
    ($self:ident, $kind:ident { $($f:ident $(: $v:expr)?),* } @ $span:expr) => {
      Stmt::new(StmtKind::$kind($self.alloc($kind { $($f $(: $v)?),* })), Span::from($span))
    };
  }

  impl<'arena, 'src> Parser<'arena, 'src> {
    pub(super) fn stmt(&mut self) -> Result<Stmt<'arena, 'src>> {
      match self.current().kind {
        KwLet => self.let_(),
        KwIf => self.if_(),
        KwLoop => self.loop_(),
        KwFn => self.func(),
        KwBreak => self.break_(),
        KwContinue => self.continue_(),
        BrkCurlyL => self.top_level_block(),
        _ => self.assign(),
      }
    }

    fn let_(&mut self) -> Result<Stmt<'arena, 'src>> {
      self.expect(KwLet)?;
      let start = self.previous().span.start;
      let name = self.ident()?;
      let value = if self.eat(OpEqual)? {
        Some(self.expr()?)
      } else {
        None
      };
      let end = self.previous().span.end;
      Ok(mk!(self, Let { name, value } @ start..end))
    }

    fn if_(&mut self) -> Result<Stmt<'arena, 'src>> {
      self.expect(KwIf)?;
      let start = self.previous().span.start;
      let cond = self.expr()?;
      let body = self.block()?;
      let tail = if self.eat(KwElse)? {
        Some(self.block()?)
      } else {
        None
      };
      let end = self.previous().span.end;
      Ok(mk!(self, If { cond, body, tail } @ start..end))
    }

    fn loop_(&mut self) -> Result<Stmt<'arena, 'src>> {
      self.expect(KwLoop)?;
      let start = self.previous().span.start;
      let body = self.block()?;
      let end = self.previous().span.end;
      Ok(mk!(self, Loop { body } @ start..end))
    }

    fn func(&mut self) -> Result<Stmt<'arena, 'src>> {
      self.expect(KwFn)?;
      let start = self.previous().span.start;
      let name = Some(self.ident()?); // required in stmt
      let params = if self.at(BrkParenL) {
        Some(self.param_list()?.into_bump_slice())
      } else {
        None
      }
      .unwrap_or(&[]);
      let body = self.block()?;
      let end = self.previous().span.end;
      Ok(mk!(self, Func { name, params, body } @ start..end))
    }

    fn break_(&mut self) -> Result<Stmt<'arena, 'src>> {
      self.expect(KwBreak)?;
      let span = self.previous().span;
      Ok(mk!(self, Break @ span))
    }

    fn continue_(&mut self) -> Result<Stmt<'arena, 'src>> {
      self.expect(KwContinue)?;
      let span = self.previous().span;
      Ok(mk!(self, Continue @ span))
    }

    fn top_level_block(&mut self) -> Result<Stmt<'arena, 'src>> {
      let start = self.current().span.start;
      let block = self.block()?;
      let end = self.previous().span.end;
      Ok(mk!(self, Block(block) @ start..end))
    }

    fn assign(&mut self) -> Result<Stmt<'arena, 'src>> {
      let target = self.expr()?;

      if let Some(kind) = self.assign_target()? {
        let value = self.expr()?;
        return self.unwrap_assign(target, value, kind);
      }

      let span = target.span;
      Ok(mk!(self, Expr(target) @ span))
    }

    fn assign_target(&mut self) -> Result<Option<AssignKind>> {
      let kind = match self.current().kind {
        OpEqual => AssignKind::Bare,
        OpPlusEqual => AssignKind::Add,
        OpMinusEqual => AssignKind::Sub,
        OpSlashEqual => AssignKind::Div,
        OpStarEqual => AssignKind::Mul,
        OpPercentEqual => AssignKind::Rem,
        OpStarStarEqual => AssignKind::Pow,
        _ => return Ok(None),
      };
      self.bump()?;
      Ok(Some(kind))
    }

    fn unwrap_assign(
      &mut self,
      target: Expr<'arena, 'src>,
      value: Expr<'arena, 'src>,
      kind: AssignKind,
    ) -> Result<Stmt<'arena, 'src>> {
      let span = Span::from(target.span.start..value.span.end);
      let assign = match target.kind {
        ExprKind::GetVar(e) => ExprKind::SetVar(self.alloc(SetVar {
          name: e.name,
          value,
          kind,
        })),
        ExprKind::GetField(e) => ExprKind::SetField(self.alloc(SetField {
          target: e.target,
          key: e.key,
          value,
          kind,
        })),
        ExprKind::GetIndex(e) => ExprKind::SetIndex(self.alloc(SetIndex {
          target: e.target,
          index: e.index,
          value,
          kind,
        })),
        _ => return Err(err!(self, @span, "invalid assignment target")),
      };
      let expr = Expr::new(assign, span);
      Ok(mk!(self, Expr(expr) @ span))
    }
  }
}

mod expr {
  use bumpalo::vec;

  use super::*;

  macro_rules! mk {
    ($self:ident, $kind:ident @ $span:expr) => {
      Expr::new(ExprKind::$kind, Span::from($span))
    };
    ($self:ident, $kind:ident ( $inner:expr ) @ $span:expr) => {
      Expr::new(ExprKind::$kind($self.alloc($inner)), Span::from($span))
    };
    ($self:ident, $kind:ident { $($f:ident $(: $v:expr)?),* } @ $span:expr) => {
      Expr::new(ExprKind::$kind($self.alloc($kind { $($f $(: $v)?),* })), Span::from($span))
    };
  }

  impl<'arena, 'src> Parser<'arena, 'src> {
    pub(super) fn expr(&mut self) -> Result<Expr<'arena, 'src>> {
      self.or()
    }

    fn or(&mut self) -> Result<Expr<'arena, 'src>> {
      let mut lhs = self.and()?;
      while !self.end() && self.eat(OpPipePipe)? {
        let rhs = self.and()?;
        let span = lhs.span.start..rhs.span.end;
        lhs = mk!(self, Binary { op: BinaryOp::Or, lhs, rhs } @ span);
      }
      Ok(lhs)
    }

    fn and(&mut self) -> Result<Expr<'arena, 'src>> {
      let mut lhs = self.eq()?;
      while !self.end() && self.eat(OpAndAnd)? {
        let rhs = self.eq()?;
        let span = lhs.span.start..rhs.span.end;
        lhs = mk!(self, Binary { op: BinaryOp::And, lhs, rhs } @ span);
      }
      Ok(lhs)
    }

    fn eq(&mut self) -> Result<Expr<'arena, 'src>> {
      let mut lhs = self.cmp()?;
      while !self.end() {
        let op = match self.current().kind {
          OpEqualEqual => BinaryOp::Eq,
          OpBangEqual => BinaryOp::Ne,
          _ => break,
        };
        self.bump()?;
        let rhs = self.cmp()?;
        let span = lhs.span.start..rhs.span.end;
        lhs = mk!(self, Binary { op, lhs, rhs } @ span);
      }
      Ok(lhs)
    }

    fn cmp(&mut self) -> Result<Expr<'arena, 'src>> {
      let mut lhs = self.term()?;
      while !self.end() {
        let op = match self.current().kind {
          OpMore => BinaryOp::Gt,
          OpMoreEqual => BinaryOp::Ge,
          OpLess => BinaryOp::Lt,
          OpLessEqual => BinaryOp::Le,
          _ => break,
        };
        self.bump()?;
        let rhs = self.term()?;
        let span = lhs.span.start..rhs.span.end;
        lhs = mk!(self, Binary { op, lhs, rhs } @ span);
      }
      Ok(lhs)
    }

    fn term(&mut self) -> Result<Expr<'arena, 'src>> {
      let mut lhs = self.factor()?;
      while !self.end() {
        let op = match self.current().kind {
          OpPlus => BinaryOp::Add,
          OpMinus => BinaryOp::Sub,
          _ => break,
        };
        self.bump()?;
        let rhs = self.factor()?;
        let span = lhs.span.start..rhs.span.end;
        lhs = mk!(self, Binary { op, lhs, rhs } @ span);
      }
      Ok(lhs)
    }

    fn factor(&mut self) -> Result<Expr<'arena, 'src>> {
      let mut lhs = self.power()?;
      while !self.end() {
        let op = match self.current().kind {
          OpStar => BinaryOp::Mul,
          OpSlash => BinaryOp::Div,
          OpPercent => BinaryOp::Rem,
          _ => break,
        };
        self.bump()?;
        let rhs = self.power()?;
        let span = lhs.span.start..rhs.span.end;
        lhs = mk!(self, Binary { op, lhs, rhs } @ span);
      }
      Ok(lhs)
    }

    fn power(&mut self) -> Result<Expr<'arena, 'src>> {
      let mut lhs = self.prefix()?;
      while !self.end() && self.eat(OpStarStar)? {
        let rhs = self.prefix()?;
        let span = lhs.span.start..rhs.span.end;
        lhs = mk!(self, Binary { op: BinaryOp::Pow, lhs, rhs } @ span);
      }
      Ok(lhs)
    }

    fn prefix(&mut self) -> Result<Expr<'arena, 'src>> {
      let op = match self.current().kind {
        OpMinus => UnaryOp::Min,
        OpBang => UnaryOp::Not,
        _ => return self.postfix(),
      };
      self.bump()?;
      let start = self.previous().span.start;
      let rhs = self.prefix()?;
      let end = rhs.span.end;
      Ok(mk!(self, Unary { op, rhs } @ start..end))
    }

    fn postfix(&mut self) -> Result<Expr<'arena, 'src>> {
      let mut expr = self.primary()?;
      while !self.end() {
        match self.current().kind {
          BrkParenL => self.call(&mut expr)?,
          BrkSquareL => self.index(&mut expr)?,
          OpDot => self.field(&mut expr)?,
          _ => break,
        }
      }
      Ok(expr)
    }

    fn call(&mut self, expr: &mut Expr<'arena, 'src>) -> Result<()> {
      let args = self.args()?.into_bump_slice();
      let span = expr.span.start..self.previous().span.end;
      *expr = mk!(self, Call { target: *expr, args } @ span);
      Ok(())
    }

    fn args(&mut self) -> Result<Vec<'arena, Expr<'arena, 'src>>> {
      self.expect(BrkParenL)?;
      let mut list = vec![in self.arena];
      if !self.end() && !self.at(BrkParenR) {
        list.push(self.expr()?);
        while !self.end() && self.eat(TokComma)? && !self.at(BrkParenR) {
          list.push(self.expr()?);
        }
      }
      self.expect(BrkParenR)?;
      Ok(list)
    }

    fn index(&mut self, expr: &mut Expr<'arena, 'src>) -> Result<()> {
      self.expect(BrkSquareL)?;
      let index = self.expr()?;
      self.expect(BrkSquareR)?;
      let target = self.alloc(*expr);
      let index = self.alloc(index);
      let span = expr.span.start..self.previous().span.end;
      *expr = mk!(self, GetIndex { target, index } @ span);
      Ok(())
    }

    fn field(&mut self, expr: &mut Expr<'arena, 'src>) -> Result<()> {
      self.expect(OpDot)?;
      let key = match self.current().kind {
        TokIdent => Key::Ident(self.ident()?),
        LitInt => Key::Int(self.int()? as usize),
        _ => return Err(self.error("invalid key", self.current().span)),
      };
      let target = self.alloc(*expr);
      let key = self.alloc(key);
      let span = expr.span.start..self.previous().span.end;
      *expr = mk!(self, GetField { target, key } @ span);
      Ok(())
    }

    fn primary(&mut self) -> Result<Expr<'arena, 'src>> {
      match self.current().kind {
        LitInt => self.lit_int(),
        LitFloat => self.lit_float(),
        LitBool => self.lit_bool(),
        LitNone => self.lit_none(),
        LitString => self.lit_str(),
        LitRecord => self.lit_record(),
        LitList => self.lit_list(),
        LitTuple => self.lit_tuple(),
        KwFn => self.lit_func(),
        KwIf => self.lit_if(),
        BrkCurlyL => self.lit_block(),
        TokIdent => self.var(),
        _ => Err(self.error(format_args!("unexpected token"), self.current().span)),
      }
    }

    fn int(&mut self) -> Result<i32> {
      self.expect(LitInt)?;
      self.lex.lexeme(self.previous()).parse().map_err(|e| {
        self.error(
          format_args!("invalid int literal: {e}"),
          self.previous().span,
        )
      })
    }

    fn lit_int(&mut self) -> Result<Expr<'arena, 'src>> {
      let inner = Lit::Int(self.int()?);
      let span = self.previous().span;
      Ok(mk!(self, Lit(inner) @ span))
    }

    fn lit_float(&mut self) -> Result<Expr<'arena, 'src>> {
      self.expect(LitFloat)?;
      let value = self.lex.lexeme(self.previous()).parse().map_err(|e| {
        self.error(
          format_args!("invalid float literal: {e}"),
          self.previous().span,
        )
      })?;
      let inner = Lit::Float(value);
      let span = self.previous().span;
      Ok(mk!(self, Lit(inner) @ span))
    }

    fn lit_bool(&mut self) -> Result<Expr<'arena, 'src>> {
      self.expect(LitBool)?;
      let value = match self.lex.lexeme(self.previous()) {
        "true" => true,
        "false" => false,
        _ => unreachable!("invalid bool literal"),
      };
      let inner = Lit::Bool(value);
      let span = self.previous().span;
      Ok(mk!(self, Lit(inner) @ span))
    }

    fn lit_none(&mut self) -> Result<Expr<'arena, 'src>> {
      self.expect(LitNone)?;
      let inner = Lit::None;
      let span = self.previous().span;
      Ok(mk!(self, Lit(inner) @ span))
    }

    fn lit_str(&mut self) -> Result<Expr<'arena, 'src>> {
      self.expect(LitString)?;
      let value = self.lex.lexeme(self.previous());
      let value = value.strip_prefix('"').expect("invalid string literal");
      let value = value.strip_suffix('"').expect("invalid string literal");
      let inner = Lit::String(value);
      let span = self.previous().span;
      Ok(mk!(self, Lit(inner) @ span))
    }

    fn lit_record(&mut self) -> Result<Expr<'arena, 'src>> {
      self.expect(LitRecord)?;
      let mut value = vec![in self.arena];
      if !self.end() && !self.at(BrkCurlyR) {
        value.push(self.member()?);
        while !self.end() && self.eat(TokComma)? && !self.at(BrkCurlyR) {
          value.push(self.member()?);
        }
      }
      self.expect(BrkCurlyR)?;

      let inner = Lit::Record(value.into_bump_slice());
      let span = self.previous().span;
      Ok(mk!(self, Lit(inner) @ span))
    }

    fn member(&mut self) -> Result<(Ident<'src>, Expr<'arena, 'src>)> {
      let name = self.ident()?;
      let value = if self.eat(TokColon)? {
        self.expr()?
      } else {
        mk!(self, GetVar { name } @ name.span)
      };
      Ok((name, value))
    }

    fn lit_list(&mut self) -> Result<Expr<'arena, 'src>> {
      self.expect(LitList)?;
      let mut value = vec![in self.arena];
      if !self.end() && !self.at(BrkSquareR) {
        value.push(self.expr()?);
        while !self.end() && self.eat(TokComma)? && !self.at(BrkSquareR) {
          value.push(self.expr()?);
        }
      }
      self.expect(BrkSquareR)?;

      let inner = Lit::List(value.into_bump_slice());
      let span = self.previous().span;
      Ok(mk!(self, Lit(inner) @ span))
    }

    fn lit_tuple(&mut self) -> Result<Expr<'arena, 'src>> {
      self.expect(LitTuple)?;
      let mut value = vec![in self.arena];
      if !self.end() && !self.at(BrkParenR) {
        value.push(self.expr()?);
        while !self.end() && self.eat(TokComma)? && !self.at(BrkParenR) {
          value.push(self.expr()?);
        }
      }
      self.expect(BrkParenR)?;

      let inner = Lit::Tuple(value.into_bump_slice());
      let span = self.previous().span;
      Ok(mk!(self, Lit(inner) @ span))
    }

    fn lit_func(&mut self) -> Result<Expr<'arena, 'src>> {
      self.expect(KwFn)?;
      let start = self.previous().span.start;
      let name = if self.at(TokIdent) {
        Some(self.ident()?)
      } else {
        None
      };
      let params = if self.at(BrkParenL) {
        Some(self.param_list()?.into_bump_slice())
      } else {
        None
      }
      .unwrap_or(&[]);
      let body = self.block()?;
      let end = self.previous().span.end;
      Ok(mk!(self, Func { name, params, body } @ start..end))
    }

    fn lit_if(&mut self) -> Result<Expr<'arena, 'src>> {
      self.expect(KwIf)?;
      let start = self.previous().span.start;
      let cond = self.expr()?;
      let body = self.block()?;
      self.expect(KwElse)?;
      let tail = Some(self.block()?);
      let end = self.previous().span.end;
      Ok(mk!(self, If { cond, body, tail } @ start..end))
    }

    fn lit_block(&mut self) -> Result<Expr<'arena, 'src>> {
      let start = self.current().span.start;
      let block = self.block()?;
      let end = self.previous().span.end;
      Ok(mk!(self, Block(block) @ start..end))
    }

    fn var(&mut self) -> Result<Expr<'arena, 'src>> {
      let name = self.ident()?;
      let span = self.previous().span;
      Ok(mk!(self, GetVar { name } @ span))
    }
  }
}

fn num_digits(v: usize) -> usize {
  use std::iter::successors;

  successors(Some(v), |&n| (n >= 10).then_some(n / 10)).count()
}
