#![deny(unused_must_use)]
#![allow(dead_code, clippy::needless_update)]

use beef::lean::Cow;

use self::indent::IndentStack;
use super::ast;
use super::lexer::TokenKind::*;
use super::lexer::{Lexer, Token, TokenKind};
use crate::ctx::Context;
use crate::error::{Error, Result};
use crate::span::Span;

// TODO: `is` and `in`
// TODO: `async`/`await` - maybe post-MVP

pub fn parse(cx: Context, src: &str) -> Result<ast::Module, Vec<Error>> {
  let lexer = Lexer::new(src);
  let parser = Parser::new(cx, lexer);
  parser.module()
}

#[derive(Clone)]
struct State {
  ignore_indent: bool,
  current_loop: Option<()>,
  current_func: Option<Func>,
  current_class: Option<Class>,
}

impl State {
  pub fn with_ignore_indent(&self) -> Self {
    Self {
      ignore_indent: true,
      current_loop: self.current_loop,
      current_func: self.current_func,
      current_class: self.current_class,
    }
  }

  pub fn with_class(has_super: bool) -> Self {
    Self {
      ignore_indent: false,
      current_loop: None,
      current_func: None,
      current_class: Some(Class { has_super }),
    }
  }

  pub fn with_func(&self, has_self: bool) -> Self {
    Self {
      ignore_indent: false,
      current_loop: None,
      current_func: Some(Func {
        has_yield: false,
        has_self,
      }),
      current_class: self.current_class,
    }
  }

  pub fn with_loop(&self) -> Self {
    Self {
      ignore_indent: false,
      current_loop: Some(()),
      current_func: self.current_func,
      current_class: self.current_class,
    }
  }
}

#[derive(Clone, Copy)]
struct Class {
  has_super: bool,
}

#[derive(Clone, Copy)]
struct Func {
  has_yield: bool,
  has_self: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for State {
  fn default() -> Self {
    Self {
      ignore_indent: false,
      current_loop: None,
      current_func: None,
      current_class: None,
    }
  }
}

#[allow(clippy::derivable_impls)]
impl Default for Func {
  fn default() -> Self {
    Self {
      has_yield: false,
      has_self: false,
    }
  }
}

struct Parser<'src> {
  cx: Context,
  module: ast::Module<'src>,
  lex: Lexer<'src>,
  errors: Vec<Error>,
  indent: IndentStack,
  state: State,
}

impl<'src> Parser<'src> {
  fn new(cx: Context, lex: Lexer<'src>) -> Self {
    Self {
      cx,
      module: ast::Module::new(),
      lex,
      errors: Vec::new(),
      indent: IndentStack::new(),
      state: State::default(),
    }
  }

  fn no_indent(&self) -> Result<()> {
    let token = self.current();
    if self.state.ignore_indent || token.is(Tok_Eof) || token.ws.is_none() {
      Ok(())
    } else {
      Err(self.cx.error("invalid indentation", token.span))
    }
  }

  fn indent_eq(&self) -> Result<()> {
    let token = self.current();
    if self.state.ignore_indent
      || token.is(Tok_Eof)
      || matches!(token.ws, Some(n) if self.indent.is_eq(n))
    {
      Ok(())
    } else {
      Err(self.cx.error("invalid indentation", token.span))
    }
  }

  fn indent_gt(&mut self) -> Result<()> {
    let token = self.current();
    if self.state.ignore_indent
      || token.is(Tok_Eof)
      || matches!(token.ws, Some(n) if self.indent.is_gt(n))
    {
      self.indent.push(token.ws.unwrap());
      Ok(())
    } else {
      Err(self.cx.error("invalid indentation", token.span))
    }
  }

  fn dedent(&mut self) -> Result<()> {
    let token = self.current();
    if self.state.ignore_indent
      || token.is(Tok_Eof)
      || matches!(token.ws, Some(n) if self.indent.is_lt(n))
    {
      self.indent.pop();
      Ok(())
    } else {
      Err(self.cx.error("invalid indentation", token.span))
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
      Err(
        self
          .cx
          .error(format!("expected `{}`", kind.name()), self.current().span),
      )
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
      self.errors.push(self.cx.error(
        format!("invalid token `{}`", self.lex.lexeme(self.current())),
        self.current().span,
      ));
      self.lex.bump();
    }
    self.previous()
  }

  /// Calls `f` in the context of `state`.
  /// `state` is used only for the duration of the call to `f`.
  #[inline]
  fn with_state<T>(
    &mut self,
    mut state: State,
    f: impl FnOnce(&mut Self) -> Result<T>,
  ) -> Result<T> {
    std::mem::swap(&mut self.state, &mut state);
    let res = f(self);
    std::mem::swap(&mut self.state, &mut state);
    res
  }

  #[inline]
  fn with_state2<T>(
    &mut self,
    mut state: State,
    f: impl FnOnce(&mut Self) -> Result<T>,
  ) -> Result<(State, T)> {
    std::mem::swap(&mut self.state, &mut state);
    let res = f(self);
    std::mem::swap(&mut self.state, &mut state);
    Ok((state, res?))
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
        Kw_Import | Kw_From | Kw_Fn | Kw_Class | Kw_For | Kw_While | Kw_Loop | Kw_If => break,
        // handle any errors
        Tok_Error => self.errors.push(self.cx.error(
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

impl<'a> Parser<'a> {
  // On average, a single parse_XXX() method consumes between 10 and 700 bytes of
  // stack space. Assuming ~50 recursive calls per dive and 700 bytes of stack
  // space per call, we'll require 50 * 700 = 35k bytes of stack space in order
  // to dive. For future proofing, we round this value up to 64k bytes.
  const MINIMUM_STACK_REQUIRED: usize = 64_000;

  // On WASM, remaining_stack() will always return None. Stack overflow panics
  // are converted to exceptions and handled by the host, which means a
  // `try { ... } catch { ... }` around a call to one of the Hebi compiler
  // functions would be enough to properly handle this case.
  #[cfg(any(target_family = "wasm", not(feature = "__check_recursion_limit")))]
  fn check_recursion_limit(&self, _span: Span) -> Result<(), Error> {
    Ok(())
  }

  #[cfg(all(not(target_family = "wasm"), feature = "__check_recursion_limit"))]
  fn check_recursion_limit(&self, span: Span) -> Result<()> {
    if stacker::remaining_stack()
      .map(|available| available > Self::MINIMUM_STACK_REQUIRED)
      .unwrap_or(true)
    {
      Ok(())
    } else {
      Err(self.cx.error("nesting limit reached", span))
    }
  }
}

#[cfg(test)]
mod tests;
