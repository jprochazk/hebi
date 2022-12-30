#![allow(non_camel_case_types)]

use std::borrow::Borrow;
use std::fmt;
use std::mem::discriminant;
use std::ops::Range;

use logos::Logos;
use span::Span;

#[derive(Clone, Debug)]
pub struct Token {
  pub ws: Option<u64>,
  pub kind: TokenKind,
  pub span: Span,
}

impl Token {
  pub fn is(&self, kind: impl Borrow<TokenKind>) -> bool {
    discriminant(&self.kind) == discriminant(kind.borrow())
  }
}

#[derive(Clone)]
pub struct Lexer<'src> {
  src: &'src str,
  inner: logos::Lexer<'src, TokenKind>,
  previous: Token,
  current: Token,
  eof: Token,
}

impl<'src> Lexer<'src> {
  pub fn new(src: &'src str) -> Self {
    let end = src.len();
    let eof = Token {
      ws: None,
      span: (end..end).into(),
      kind: TokenKind::Tok_Eof,
    };

    let mut lex = Self {
      src,
      inner: TokenKind::lexer(src),
      previous: eof.clone(),
      current: eof.clone(),
      eof,
    };
    lex.bump();

    lex
  }

  #[inline]
  pub fn previous(&self) -> &Token {
    &self.previous
  }

  #[inline]
  pub fn current(&self) -> &Token {
    &self.current
  }

  #[inline]
  pub fn eof(&self) -> &Token {
    &self.eof
  }

  #[inline]
  pub fn lexeme(&self, token: &Token) -> &'src str {
    &self.src[Range::from(token.span)]
  }

  #[inline]
  pub fn bump(&mut self) {
    std::mem::swap(&mut self.previous, &mut self.current);

    self.current = self.next_token().unwrap_or(self.eof.clone());
  }

  fn next_token(&mut self) -> Option<Token> {
    let lexer = &mut self.inner;
    let mut ws = None;
    while let Some(kind) = lexer.next() {
      let lexeme = lexer.slice();
      let span = lexer.span().into();

      match kind {
        // Filter
        TokenKind::_Tok_Whitespace | TokenKind::_Tok_Comment => {}
        // Measure indentation
        TokenKind::_Tok_Indent => ws = Some(measure_indent(lexeme)),
        // Return token
        _ => return Some(Token { ws, kind, span }),
      }
    }

    None
  }
}

// When adding a token, if it is matched using `token` directive only,
// then it should also be added to the `known` module below.
#[derive(Clone, Copy, Debug, Logos, PartialEq)]
pub enum TokenKind {
  // Keywords
  #[token("use")]
  Kw_Use,
  #[token("as")]
  Kw_As,
  #[token("pub")]
  Kw_Pub,
  #[token("fn")]
  Kw_Fn,
  #[token("yield")]
  Kw_Yield,
  #[token("class")]
  Kw_Class,
  #[token("for")]
  Kw_For,
  #[token("in")]
  Kw_In,
  #[token("while")]
  Kw_While,
  #[token("loop")]
  Kw_Loop,
  #[token("return")]
  Kw_Return,
  #[token("break")]
  Kw_Break,
  #[token("continue")]
  Kw_Continue,
  #[token("if")]
  Kw_If,
  #[token("elif")]
  Kw_Elif,
  #[token("else")]
  Kw_Else,
  #[token("pass")]
  Kw_Pass,

  // Brackets
  #[token("{")]
  Brk_CurlyL,
  #[token("}")]
  Brk_CurlyR,
  #[token("(")]
  Brk_ParenL,
  #[token(")")]
  Brk_ParenR,
  #[token("[")]
  Brk_SquareL,
  #[token("]")]
  Brk_SquareR,

  // Misc characters
  #[token(".")]
  Op_Dot,
  #[token(",")]
  Tok_Comma,
  #[token(";")]
  Tok_Semicolon,
  #[token(":")]
  Tok_Colon,
  #[token("?")]
  Tok_Question,

  // Equals operators
  #[token("=")]
  Op_Equal,
  #[token("==")]
  Op_EqualEqual,
  #[token("+=")]
  Op_PlusEqual,
  #[token("-=")]
  Op_MinusEqual,
  #[token("/=")]
  Op_SlashEqual,
  #[token("*=")]
  Op_StarEqual,
  #[token("%=")]
  Op_PercentEqual,
  #[token("**=")]
  Op_StarStarEqual,
  #[token("??=")]
  Op_QuestionQuestionEqual,
  #[token("!=")]
  Op_BangEqual,
  #[token(":=")]
  Op_ColonEqual,

  // Operators
  #[token("+")]
  Op_Plus,
  #[token("-")]
  Op_Minus,
  #[token("/")]
  Op_Slash,
  #[token("*")]
  Op_Star,
  #[token("%")]
  Op_Percent,
  #[token("**")]
  Op_StarStar,
  #[token("??")]
  Op_QuestionQuestion,
  #[token("!")]
  Op_Bang,
  #[token(">")]
  Op_More,
  #[token(">=")]
  Op_MoreEqual,
  #[token("<")]
  Op_Less,
  #[token("<=")]
  Op_LessEqual,
  #[token("||")]
  Op_PipePipe,
  #[token("&&")]
  Op_AndAnd,
  #[token("..")]
  Op_Range,
  #[token("..=")]
  Op_RangeInc,

  // Literals
  /// `null`
  #[token("null")]
  Lit_Null,
  /// `0`, `1.0`, `5e10`, etc.
  #[regex(r"[0-9]+(\.[0-9]+)?([Ee][+-]?[0-9]+)?")]
  Lit_Number,
  /// `true` or `false`
  #[token("true")]
  #[token("false")]
  Lit_Bool,
  #[regex(r#""([^"\\]|\\.)*""#)]
  Lit_String,
  /// `a`, `b_c`, `__x0`, etc.
  #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
  Lit_Ident,

  #[doc(hidden)]
  #[regex(r"(\r?\n)+[ ]*", priority = 10)]
  _Tok_Indent,
  #[doc(hidden)]
  #[regex(r"[ \n\r]+")]
  _Tok_Whitespace,
  #[doc(hidden)]
  #[regex(r"#[^\n]*")]
  _Tok_Comment,

  #[error]
  Tok_Error,
  Tok_Eof,
}

fn measure_indent(s: &str) -> u64 {
  let pos = s.rfind('\n').unwrap_or(0);
  (s.len() - pos - 1) as u64
}

pub struct Tokens<'src>(pub Lexer<'src>);

impl<'src> Iterator for Tokens<'src> {
  type Item = (&'src str, Token);

  fn next(&mut self) -> Option<Self::Item> {
    let token = self.0.current().clone();
    self.0.bump();
    if !token.is(TokenKind::Tok_Eof) {
      Some((self.0.lexeme(&token), token))
    } else {
      None
    }
  }
}

pub struct DebugToken<'src>(pub Token, pub &'src str);
impl<'src> fmt::Debug for DebugToken<'src> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let ws = self
      .0
      .ws
      .map(|v| v.to_string())
      .unwrap_or_else(|| "_".to_owned());
    let kind = self.0.kind;
    let span = self.0.span;
    let lexeme = self.1;
    if let TokenKind::Lit_Ident = self.0.kind {
      write!(f, "(>{ws} {kind:?} `{lexeme}` @{span})")
    } else {
      write!(f, "(>{ws} {kind:?} @{span})")
    }
  }
}

#[cfg(test)]
mod tests;
