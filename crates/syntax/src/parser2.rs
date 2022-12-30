#![deny(unused_must_use)]
#![allow(dead_code, clippy::needless_update)]

use beef::lean::Cow;
use span::{Span, Spanned};

use self::indent::IndentStack;
use crate::lexer2::TokenKind::*;
use crate::lexer2::{Lexer, Token, TokenKind};
use crate::{ast, Error, Result};

// https://github.com/ezclap-tv/mu-lang/blob/v2/crates/syntax/src/parser.rs
// https://github.com/ezclap-tv/mu-lang/blob/v2/crates/syntax/src/lexer.rs

// TODO: check recursion limit when parsing block

pub fn parse(src: &str) -> Result<ast::Module, Vec<Error>> {
  let lexer = Lexer::new(src);
  let parser = Parser::new(lexer);
  parser.module()
}

struct Context {
  ignore_indent: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for Context {
  fn default() -> Self {
    Self {
      ignore_indent: false,
    }
  }
}

struct Parser<'src> {
  lex: Lexer<'src>,
  errors: Vec<Error>,
  indent: IndentStack,
  ctx: Context,
}

impl<'src> Parser<'src> {
  fn new(lex: Lexer<'src>) -> Self {
    Self {
      lex,
      errors: Vec::new(),
      indent: IndentStack::new(),
      ctx: Context::default(),
    }
  }

  fn no_indent(&self) -> Result<()> {
    let token = self.current();
    if self.ctx.ignore_indent || token.is(Tok_Eof) || token.ws.is_none() {
      Ok(())
    } else {
      Err(Error::new("invalid indentation", token.span))
    }
  }

  fn indent_eq(&self) -> Result<()> {
    let token = self.current();
    if self.ctx.ignore_indent
      || token.is(Tok_Eof)
      || matches!(token.ws, Some(n) if self.indent.is_indent_eq(n))
    {
      Ok(())
    } else {
      Err(Error::new("invalid indentation", token.span))
    }
  }

  fn indent_gt(&mut self) -> Result<()> {
    let token = self.current();
    if self.ctx.ignore_indent
      || token.is(Tok_Eof)
      || matches!(token.ws, Some(n) if self.indent.is_indent_gt(n))
    {
      self.indent.push_indent(token.ws.unwrap());
      Ok(())
    } else {
      Err(Error::new("invalid indentation", token.span))
    }
  }

  fn dedent(&mut self) -> Result<()> {
    let token = self.current();
    if self.ctx.ignore_indent
      || token.is(Tok_Eof)
      || matches!(token.ws, Some(n) if self.indent.is_indent_lt(n))
    {
      self.indent.pop_indent();
      Ok(())
    } else {
      Err(Error::new("invalid indentation", token.span))
    }
  }

  #[inline]
  fn previous(&self) -> &Token {
    self.lex.previous()
  }

  #[inline]
  fn current(&self) -> &Token {
    self.lex.current()
  }

  #[inline]
  fn expect(&mut self, kind: TokenKind) -> Result<()> {
    if self.bump_if(kind) {
      Ok(())
    } else {
      Err(Error::new(
        format!("expected `{}`", kind.name()),
        self.current().span,
      ))
    }
  }

  #[inline]
  fn bump_if(&mut self, kind: TokenKind) -> bool {
    if self.current().is(kind) {
      self.bump();
      true
    } else {
      false
    }
  }

  /// Move forward by one token, returning the previous one.
  #[inline]
  fn bump(&mut self) -> &Token {
    self.lex.bump();
    while self.current().is(Tok_Error) {
      self.errors.push(Error::new(
        format!("invalid token `{}`", self.lex.lexeme(self.current())),
        self.current().span,
      ));
      self.lex.bump();
    }
    self.previous()
  }

  /// Calls `f` in the context `ctx`.
  /// `ctx` is used only for the duration of the call to `f`.
  #[inline]
  fn with_ctx<T>(&mut self, ctx: Context, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
    let mut prev_ctx = std::mem::replace(&mut self.ctx, ctx);
    let res = f(self);
    std::mem::swap(&mut self.ctx, &mut prev_ctx);
    res
  }

  /// Calls `f` and wraps the returned value in a span that encompasses the
  /// entire sequence of tokens parsed within `f`.
  #[inline]
  fn span<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<Spanned<T>> {
    let start = self.current().span;
    f(self).map(|value| {
      let end = self.previous().span;
      Spanned::new(start.join(end), value)
    })
  }

  fn sync(&mut self) {
    self.bump();
    while !self.current().is(Tok_Eof) {
      // break when exiting a block (dedent)
      if self.dedent().is_ok() {
        break;
      }

      match self.current().kind {
        // break on keywords that begin statements
        Kw_Use | Kw_Fn | Kw_Class | Kw_For | Kw_While | Kw_Loop | Kw_If => break,
        // handle any errors
        Tok_Error => self.errors.push(Error::new(
          format!("invalid token `{}`", self.lex.lexeme(self.current())),
          self.current().span,
        )),
        _ => {}
      }

      self.bump();
    }
  }
}

mod indent;
mod common {

  use super::*;

  impl<'src> Parser<'src> {
    pub(super) fn ident(&mut self) -> Result<ast::Ident<'src>> {
      self.expect(Lit_Ident)?;
      Ok(Spanned::new(
        self.previous().span,
        Cow::from(self.lex.lexeme(self.previous())),
      ))
    }
  }
}
mod module {
  use super::*;

  impl<'src> Parser<'src> {
    pub(super) fn module(mut self) -> Result<ast::Module<'src>, Vec<Error>> {
      let mut module = ast::Module::new();

      while !self.current().is(Tok_Eof) {
        eprintln!("{:?}", self.current());
        if let Err(e) = self.top_level_stmt(&mut module) {
          self.errors.push(e);
          self.sync();
        }
      }

      if !self.errors.is_empty() {
        return Err(self.errors);
      }

      Ok(module)
    }
  }
}
mod expr {
  use super::*;

  // TODO: expr_opt -> `?expr` -> high precedence

  impl<'src> Parser<'src> {
    pub(super) fn expr(&mut self) -> Result<ast::Expr<'src>> {
      self.maybe_expr()
    }

    fn maybe_expr(&mut self) -> Result<ast::Expr<'src>> {
      let mut left = self.or_expr()?;
      while self.no_indent().is_ok() && self.bump_if(Op_QuestionQuestion) {
        self.no_indent()?;
        let right = self.or_expr()?;
        left = ast::expr_binary(
          left.span.start..right.span.end,
          ast::BinaryOp::Maybe,
          left,
          right,
        );
      }
      Ok(left)
    }

    fn or_expr(&mut self) -> Result<ast::Expr<'src>> {
      let mut left = self.and_expr()?;
      while self.no_indent().is_ok() && self.bump_if(Op_PipePipe) {
        self.no_indent()?;
        let right = self.and_expr()?;
        left = ast::expr_binary(
          left.span.start..right.span.end,
          ast::BinaryOp::Or,
          left,
          right,
        );
      }
      Ok(left)
    }

    fn and_expr(&mut self) -> Result<ast::Expr<'src>> {
      let mut left = self.eq_expr()?;
      while self.no_indent().is_ok() && self.bump_if(Op_AndAnd) {
        self.no_indent()?;
        let right = self.eq_expr()?;
        left = ast::expr_binary(
          left.span.start..right.span.end,
          ast::BinaryOp::And,
          left,
          right,
        );
      }
      Ok(left)
    }

    fn eq_expr(&mut self) -> Result<ast::Expr<'src>> {
      let mut left = self.comp_expr()?;
      while self.no_indent().is_ok() {
        let op = match self.current().kind {
          Op_EqualEqual => ast::BinaryOp::Eq,
          Op_BangEqual => ast::BinaryOp::Neq,
          _ => break,
        };
        self.bump(); // bump operator
        self.no_indent()?;
        let right = self.comp_expr()?;
        left = ast::expr_binary(left.span.start..right.span.end, op, left, right);
      }
      Ok(left)
    }

    fn comp_expr(&mut self) -> Result<ast::Expr<'src>> {
      let mut left = self.add_expr()?;
      while self.no_indent().is_ok() {
        let op = match self.current().kind {
          Op_Less => ast::BinaryOp::Less,
          Op_LessEqual => ast::BinaryOp::LessEq,
          Op_More => ast::BinaryOp::More,
          Op_MoreEqual => ast::BinaryOp::MoreEq,
          _ => break,
        };
        self.bump(); // bump operator
        self.no_indent()?;
        let right = self.add_expr()?;
        left = ast::expr_binary(left.span.start..right.span.end, op, left, right);
      }
      Ok(left)
    }

    fn add_expr(&mut self) -> Result<ast::Expr<'src>> {
      let mut left = self.mul_expr()?;
      while self.no_indent().is_ok() {
        let op = match self.current().kind {
          Op_Plus => ast::BinaryOp::Add,
          Op_Minus => ast::BinaryOp::Sub,
          _ => break,
        };
        self.bump(); // bump operator
        self.no_indent()?;
        let right = self.mul_expr()?;
        left = ast::expr_binary(left.span.start..right.span.end, op, left, right);
      }
      Ok(left)
    }

    fn mul_expr(&mut self) -> Result<ast::Expr<'src>> {
      let mut left = self.pow_expr()?;
      while self.no_indent().is_ok() {
        let op = match self.current().kind {
          Op_Star => ast::BinaryOp::Mul,
          Op_Slash => ast::BinaryOp::Div,
          Op_Percent => ast::BinaryOp::Rem,
          _ => break,
        };
        self.bump(); // bump operator
        self.no_indent()?;
        let right = self.pow_expr()?;
        left = ast::expr_binary(left.span.start..right.span.end, op, left, right);
      }
      Ok(left)
    }

    fn pow_expr(&mut self) -> Result<ast::Expr<'src>> {
      let mut left = self.unary_expr()?;
      while self.no_indent().is_ok() && self.bump_if(Op_StarStar) {
        self.no_indent()?;
        let right = self.unary_expr()?;
        left = ast::expr_binary(
          left.span.start..right.span.end,
          ast::BinaryOp::Pow,
          left,
          right,
        );
      }
      Ok(left)
    }

    fn unary_expr(&mut self) -> Result<ast::Expr<'src>> {
      let op = match self.current().kind {
        Op_Minus => ast::UnaryOp::Minus,
        Op_Plus => ast::UnaryOp::Plus,
        Op_Bang => ast::UnaryOp::Not,
        _ => return self.postfix_expr(),
      };
      self.bump(); // bump operator
      let start = self.previous().span.start;
      self.no_indent()?;
      let right = self.unary_expr()?;
      Ok(ast::expr_unary(start..right.span.end, op, right))
    }

    fn postfix_expr(&mut self) -> Result<ast::Expr<'src>> {
      let mut expr = self.primary_expr()?;
      while self.no_indent().is_ok() {
        match self.current().kind {
          Brk_ParenL => {
            let args = self.call_args()?; // bumps `(`
            expr = ast::expr_call(expr.span.start..self.previous().span.end, expr, args);
          }
          Brk_SquareL => {
            self.bump(); // bump `[`
            let key = self.expr()?;
            self.expect(Brk_SquareR)?;
            expr = ast::expr_index(expr.span.start..self.previous().span.end, expr, key);
          }
          Op_Dot => {
            self.bump(); // bump `.`
            let key = self.ident()?;
            expr = ast::expr_field(expr.span.start..key.span.end, expr, key);
          }
          _ => break,
        }
      }
      Ok(expr)
    }

    fn primary_expr(&mut self) -> Result<ast::Expr<'src>> {
      check_recursion_limit(self.current().span)?;

      if self.bump_if(Lit_Null) {
        return Ok(ast::lit2::null(self.previous().span));
      }

      if self.bump_if(Lit_Bool) {
        let token = self.previous();
        return Ok(ast::lit2::bool(token.span, self.lex.lexeme(token)));
      }

      if self.bump_if(Lit_Number) {
        let token = self.previous();
        return ast::lit2::num(token.span, self.lex.lexeme(token));
      }

      if self.bump_if(Lit_String) {
        let token = self.previous();
        return Ok(ast::lit2::str(token.span, self.lex.lexeme(token)));
      }

      if self.bump_if(Brk_SquareL) {
        let start = self.previous().span.start;

        let mut items = vec![];
        if !self.current().is(Brk_SquareR) {
          items.push(self.expr()?);
          while self.bump_if(Tok_Comma) && !self.current().is(Brk_SquareR) {
            items.push(self.expr()?);
          }
        }

        self.expect(Brk_SquareR)?;
        let end = self.previous().span.end;
        return Ok(ast::expr_array(start..end, items));
      }

      if self.bump_if(Brk_CurlyL) {
        let start = self.previous().span.start;

        let mut items = vec![];
        if !self.current().is(Brk_CurlyR) {
          items.push((self.object_key()?, self.expr()?));
          while self.bump_if(Tok_Comma) && !self.current().is(Brk_CurlyR) {
            items.push((self.object_key()?, self.expr()?));
          }
        }

        self.expect(Brk_CurlyR)?;
        let end = self.previous().span.end;
        return Ok(ast::expr_object(start..end, items));
      }

      if self.current().is(Lit_Ident) {
        return Ok(ast::expr_get_var(self.ident()?));
      }

      if self.bump_if(Brk_ParenL) {
        let ctx = Context {
          ignore_indent: true,
          ..Default::default()
        };
        let expr = self.with_ctx(ctx, |p| p.expr())?;
        self.expect(Brk_ParenR)?;
        return Ok(expr);
      }

      Err(Error::new("unexpected token", self.current().span))
    }

    fn object_key(&mut self) -> Result<ast::Expr<'src>> {
      if self.bump_if(Brk_SquareL) {
        let key = self.expr()?;
        self.expect(Brk_SquareR)?;
        Ok(key)
      } else {
        let key = ast::ident_key(self.ident()?);
        Ok(key)
      }
    }

    fn call_args(&mut self) -> Result<ast::Args<'src>> {
      let mut args = ast::Args::new();
      self.expect(Brk_ParenL)?;
      if !self.current().is(Brk_ParenR) {
        let ctx = Context {
          ignore_indent: true,
          ..Default::default()
        };
        self.with_ctx(ctx, |p| {
          let mut parsing_kw = false;
          p.call_arg_one(&mut args, &mut parsing_kw)?;
          while p.bump_if(Tok_Comma) && !p.current().is(Brk_ParenR) {
            p.call_arg_one(&mut args, &mut parsing_kw)?;
          }
          Ok(())
        })?;
      }
      self.expect(Brk_ParenR)?;
      Ok(args)
    }

    fn call_arg_one(&mut self, args: &mut ast::Args<'src>, parsing_kw: &mut bool) -> Result<()> {
      // to avoid lookahead or backtracking, `GetVar` is treated as the identifier in
      // f(<ident> = <value>)
      let expr = self.expr()?;
      if *parsing_kw {
        let expr_span = expr.span;
        let ast::ExprKind::GetVar(ident) = expr.into_inner() else {
          return Err(Error::new("keyword argument", expr_span));
        };
        self.expect(Op_Equal)?;
        let value = self.expr()?;
        args.kw(ident.name, value);
      } else if self.bump_if(Op_Equal) {
        *parsing_kw = true;
        let expr_span = expr.span;
        let ast::ExprKind::GetVar(ident) = expr.into_inner() else {
          return Err(Error::new("keyword argument", expr_span));
        };
        let value = self.expr()?;
        args.kw(ident.name, value);
      } else {
        args.pos(expr);
      }

      Ok(())
    }
  }
}
mod stmt {
  use super::*;

  impl<'src> Parser<'src> {
    pub(super) fn top_level_stmt(&mut self, module: &mut ast::Module<'src>) -> Result<()> {
      self.indent_eq()?;

      if self.bump_if(Kw_Use) {
        self.import_stmt(module)?;
      } else {
        module.body.push(self.stmt()?)
      }

      Ok(())
    }

    fn import_stmt(&mut self, module: &mut ast::Module<'src>) -> Result<()> {
      #[allow(clippy::ptr_arg)]
      fn extend_path<'src>(
        p: &Vec<ast::Ident<'src>>,
        v: ast::Ident<'src>,
      ) -> Vec<ast::Ident<'src>> {
        let mut p = p.clone();
        p.push(v);
        p
      }

      fn import_stmt_inner<'src>(
        p: &mut Parser<'src>,
        path: &Vec<ast::Ident<'src>>,
        module: &mut ast::Module<'src>,
      ) -> Result<()> {
        check_recursion_limit(p.current().span)?;

        let path = extend_path(path, p.ident()?);
        if p.bump_if(Kw_As) {
          let alias = Some(p.ident()?);
          module.imports.push(ast::Import { path, alias });
          return Ok(());
        }

        if p.bump_if(Op_Dot) {
          if p.bump_if(Brk_CurlyL) {
            import_stmt_inner(p, &path, module)?;
            while p.bump_if(Tok_Comma) && !p.current().is(Brk_CurlyR) {
              import_stmt_inner(p, &path, module)?;
            }
            p.expect(Brk_CurlyR)?;
            return Ok(());
          }

          import_stmt_inner(p, &path, module)?;
          return Ok(());
        }

        module.imports.push(ast::Import { path, alias: None });
        Ok(())
      }

      let path = vec![];
      if self.bump_if(Brk_CurlyL) {
        import_stmt_inner(self, &path, module)?;
        while self.bump_if(Tok_Comma) && !self.current().is(Brk_CurlyR) {
          import_stmt_inner(self, &path, module)?;
        }
        self.expect(Brk_CurlyR)?;
      } else {
        import_stmt_inner(self, &path, module)?;
      }

      Ok(())
    }

    fn stmt(&mut self) -> Result<ast::Stmt<'src>> {
      todo!()
    }
  }
}

// On average, a single parse_XXX() method consumes between 10 and 700 bytes of
// stack space. Assuming ~50 recursive calls per dive and 700 bytes of stack
// space per call, we'll require 50 * 700 = 35k bytes of stack space in order
// to dive. For future proofing, we round this value up to 64k bytes.
const MINIMUM_STACK_REQUIRED: usize = 64_000;

// On WASM, remaining_stack() will always return None. Stack overflow panics
// are converted to exceptions and handled by the host, which means a
// `try { ... } catch { ... }` around a call to one of the Mu compiler functions
// would be enough to properly handle this case.
#[cfg(target_family = "wasm")]
fn check_recursion_limit(_span: Span) -> Result<(), Error> {
  Ok(())
}

#[cfg(not(target_family = "wasm"))]
fn check_recursion_limit(span: Span) -> Result<()> {
  if stacker::remaining_stack()
    .map(|available| available > MINIMUM_STACK_REQUIRED)
    .unwrap_or(true)
  {
    Ok(())
  } else {
    Err(Error::new("nesting limit reached", span))
  }
}

#[cfg(test)]
mod tests;
