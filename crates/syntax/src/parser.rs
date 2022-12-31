#![deny(unused_must_use)]
#![allow(dead_code, clippy::needless_update)]

use beef::lean::Cow;
use span::{Span, Spanned};

use self::indent::IndentStack;
use crate::lexer::TokenKind::*;
use crate::lexer::{Lexer, Token, TokenKind};
use crate::{ast, Error, Result};

// https://github.com/ezclap-tv/mu-lang/blob/v2/crates/syntax/src/parser.rs
// https://github.com/ezclap-tv/mu-lang/blob/v2/crates/syntax/src/lexer.rs

// TODO: add the ability to contextualise errors

pub fn parse(src: &str) -> Result<ast::Module, Vec<Error>> {
  let lexer = Lexer::new(src);
  let parser = Parser::new(lexer);
  parser.module()
}

struct Context {
  ignore_indent: bool,
  current_loop: Option<()>,
  current_func: Option<Func>,
}

#[derive(Clone, Copy)]
struct Func {
  has_yield: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for Context {
  fn default() -> Self {
    Self {
      ignore_indent: false,
      current_loop: None,
      current_func: None,
    }
  }
}

#[allow(clippy::derivable_impls)]
impl Default for Func {
  fn default() -> Self {
    Self { has_yield: false }
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
      || matches!(token.ws, Some(n) if self.indent.is_eq(n))
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
      || matches!(token.ws, Some(n) if self.indent.is_gt(n))
    {
      self.indent.push(token.ws.unwrap());
      Ok(())
    } else {
      Err(Error::new("invalid indentation", token.span))
    }
  }

  fn dedent(&mut self) -> Result<()> {
    let token = self.current();
    if self.ctx.ignore_indent
      || token.is(Tok_Eof)
      || matches!(token.ws, Some(n) if self.indent.is_lt(n))
    {
      self.indent.pop();
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
  fn with_ctx<T>(&mut self, mut ctx: Context, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
    std::mem::swap(&mut self.ctx, &mut ctx);
    let res = f(self);
    std::mem::swap(&mut self.ctx, &mut ctx);
    res
  }

  #[inline]
  fn with_ctx2<T>(
    &mut self,
    mut ctx: Context,
    f: impl FnOnce(&mut Self) -> Result<T>,
  ) -> Result<(Context, T)> {
    std::mem::swap(&mut self.ctx, &mut ctx);
    let res = f(self);
    std::mem::swap(&mut self.ctx, &mut ctx);
    Ok((ctx, res?))
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
      // but not in an if statement, because it is composed of multiple blocks
      if self.dedent().is_ok() && ![Kw_Else, Kw_Elif].contains(&self.current().kind) {
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

mod common;
mod expr;
mod indent;
mod module;
mod stmt;

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
