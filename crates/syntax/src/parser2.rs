#![deny(unused_must_use)]

use span::{Span, Spanned};

use self::indent::IndentStack;
use crate::lexer2::TokenKind::*;
use crate::lexer2::{Lexer, Token, TokenKind};
use crate::{ast, Error, Result};

// https://github.com/ezclap-tv/mu-lang/blob/v2/crates/syntax/src/parser.rs
// https://github.com/ezclap-tv/mu-lang/blob/v2/crates/syntax/src/lexer.rs

pub fn parse(src: &str) -> Result<ast::Module> {
  let lexer = Lexer::new(src);
  let parser = Parser::new(lexer);
  parser.module()
}

struct Context {
  ignore_indent: bool,
}

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
    // TODO: in which scenarios is it safe to continue parsing?
  }
}

mod indent;
mod module {
  use super::*;

  impl<'src> Parser<'src> {
    pub fn module(mut self) -> Result<ast::Module<'src>> {
      let mut module = ast::Module::new();

      while !self.current().is(Tok_Eof) {
        if let Err(e) = self.top_level_stmt(&mut module) {
          self.errors.push(e);
          self.sync();
        }
      }

      Ok(module)
    }

    pub fn import_stmt(&mut self, module: &mut ast::Module<'src>) -> Result<()> {
      todo!()
    }
  }
}
mod stmt {
  use super::*;

  impl<'src> Parser<'src> {
    pub fn top_level_stmt(&mut self, module: &mut ast::Module<'src>) -> Result<()> {
      self.indent_eq()?;

      if self.bump_if(Kw_Use) {
        self.import_stmt(module)?;
      } else {
        module.body.push(self.stmt()?)
      }

      todo!()
    }

    pub fn stmt(&mut self) -> Result<ast::Stmt<'src>> {
      todo!()
    }
  }
}

#[cfg(test)]
mod tests;
