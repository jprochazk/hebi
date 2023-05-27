#![allow(non_camel_case_types)]

use std::borrow::Borrow;
use std::fmt;
use std::mem::{discriminant, take};
use std::ops::Range;

use logos::Logos;

use crate::span::Span;

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
  ws: Option<u64>,
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
      ws: Some(0),
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
    while let Some(kind) = lexer.next() {
      let lexeme = lexer.slice();
      let span = lexer.span().into();

      match kind {
        // Filter
        Ok(TokenKind::_Tok_Whitespace | TokenKind::_Tok_Comment) => continue,
        // Measure indentation
        Ok(TokenKind::_Tok_Indent) => {
          self.ws = Some(measure_indent(lexeme));
          continue;
        }
        // Return any other token
        Ok(kind) => {
          let token = Token {
            ws: take(&mut self.ws),
            kind,
            span,
          };
          return Some(token);
        }
        Err(_) => {
          let token = Token {
            ws: take(&mut self.ws),
            kind: TokenKind::Tok_Error,
            span,
          };
          return Some(token);
        }
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
  #[token("import")]
  Kw_Import,
  #[token("from")]
  Kw_From,
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
  #[token("self")]
  Kw_Self,
  #[token("super")]
  Kw_Super,
  #[token("for")]
  Kw_For,
  #[token("is")]
  Kw_Is,
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
  #[token("print")]
  Kw_Print,
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
  /// `none`
  #[token("none")]
  Lit_None,
  #[regex("[0-9]([0-9_]*[0-9])?", priority = 10)]
  Lit_Int,
  /// `0`, `1.0`, `5e10`, etc.
  #[regex(r"[0-9]+(\.[0-9]+)?([Ee][+-]?[0-9]+)?")]
  Lit_Float,
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

  Tok_Error,
  Tok_Eof,
}

impl TokenKind {
  pub fn name(&self) -> &'static str {
    match self {
      TokenKind::Kw_Import => "import",
      TokenKind::Kw_From => "from",
      TokenKind::Kw_As => "as",
      TokenKind::Kw_Pub => "pub",
      TokenKind::Kw_Fn => "fn",
      TokenKind::Kw_Yield => "yield",
      TokenKind::Kw_Class => "class",
      TokenKind::Kw_Self => "self",
      TokenKind::Kw_Super => "super",
      TokenKind::Kw_For => "for",
      TokenKind::Kw_Is => "is",
      TokenKind::Kw_In => "in",
      TokenKind::Kw_While => "while",
      TokenKind::Kw_Loop => "loop",
      TokenKind::Kw_Return => "return",
      TokenKind::Kw_Break => "break",
      TokenKind::Kw_Continue => "continue",
      TokenKind::Kw_Print => "print",
      TokenKind::Kw_If => "if",
      TokenKind::Kw_Elif => "elif",
      TokenKind::Kw_Else => "else",
      TokenKind::Kw_Pass => "pass",
      TokenKind::Brk_CurlyL => "{",
      TokenKind::Brk_CurlyR => "}",
      TokenKind::Brk_ParenL => "(",
      TokenKind::Brk_ParenR => ")",
      TokenKind::Brk_SquareL => "[",
      TokenKind::Brk_SquareR => "]",
      TokenKind::Op_Dot => ".",
      TokenKind::Tok_Comma => ",",
      TokenKind::Tok_Semicolon => ";",
      TokenKind::Tok_Colon => ":",
      TokenKind::Tok_Question => "?",
      TokenKind::Op_Equal => "=",
      TokenKind::Op_EqualEqual => "==",
      TokenKind::Op_PlusEqual => "+=",
      TokenKind::Op_MinusEqual => "-=",
      TokenKind::Op_SlashEqual => "/=",
      TokenKind::Op_StarEqual => "*=",
      TokenKind::Op_PercentEqual => "%=",
      TokenKind::Op_StarStarEqual => "**=",
      TokenKind::Op_QuestionQuestionEqual => "?=",
      TokenKind::Op_BangEqual => "!=",
      TokenKind::Op_ColonEqual => ":=",
      TokenKind::Op_Plus => "+",
      TokenKind::Op_Minus => "-",
      TokenKind::Op_Slash => "/",
      TokenKind::Op_Star => "*",
      TokenKind::Op_Percent => "%",
      TokenKind::Op_StarStar => "**",
      TokenKind::Op_QuestionQuestion => "??",
      TokenKind::Op_Bang => "!",
      TokenKind::Op_More => ">",
      TokenKind::Op_MoreEqual => ">=",
      TokenKind::Op_Less => "<",
      TokenKind::Op_LessEqual => "<=",
      TokenKind::Op_PipePipe => "||",
      TokenKind::Op_AndAnd => "&&",
      TokenKind::Op_Range => "..",
      TokenKind::Op_RangeInc => "..=",
      TokenKind::Lit_None => "none",
      TokenKind::Lit_Int => "int",
      TokenKind::Lit_Float => "float",
      TokenKind::Lit_Bool => "bool",
      TokenKind::Lit_String => "string",
      TokenKind::Lit_Ident => "identifier",
      TokenKind::_Tok_Indent => "<indentation>",
      TokenKind::_Tok_Whitespace => "<whitespace>",
      TokenKind::_Tok_Comment => "<comment>",
      TokenKind::Tok_Error => "<error>",
      TokenKind::Tok_Eof => "<eof>",
    }
  }
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

#[cfg(all(test, not(feature = "__miri")))]
mod tests;
