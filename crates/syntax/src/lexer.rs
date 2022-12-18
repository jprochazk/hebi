#![allow(non_camel_case_types)]

use std::fmt;

use beef::lean::Cow;
use logos::Logos;
use span::Span;

#[derive(Clone, Debug)]
pub struct Token<'src> {
  pub ws: usize,
  pub lexeme: Cow<'src, str>,
  pub kind: TokenKind,
  pub span: Span,
}

pub struct Lexer<'src> {
  tokens: Vec<Token<'src>>,
  eof: Span,
}

#[derive(Debug)]
pub struct Error {
  pub span: Span,
  pub lexeme: String,
}

impl<'src> Lexer<'src> {
  pub fn lex(src: &'src str) -> Result<Self, Vec<Error>> {
    let eof = (src.len()..src.len()).into();

    let mut ws = 0;
    let mut errors = vec![];
    let mut tokens = vec![];
    let mut lexer = logos::Lexer::<'src, TokenKind>::new(src);
    while let Some(kind) = lexer.next() {
      let lexeme = lexer.slice();
      let span = lexer.span().into();

      match kind {
        // Handle indentation
        TokenKind::_Indentation => {
          ws = lexeme.trim_start_matches(|c| c == '\n' || c == '\r').len();
          continue;
        }
        // Filter any other whitespace and comments
        TokenKind::_Whitespace | TokenKind::_Comment => continue,
        // Handle errors
        TokenKind::_Error => {
          errors.push(Error {
            lexeme: lexeme.into(),
            span,
          });
          continue;
        }
        // Any other token is stored with its preceding whitespace
        _ => {
          tokens.push(Token {
            ws,
            lexeme: lexeme.into(),
            kind,
            span,
          });
          ws = 0;
        }
      }
    }

    if !errors.is_empty() {
      Err(errors)
    } else {
      Ok(Lexer { tokens, eof })
    }
  }
}

impl<'src> peg::Parse for Lexer<'src> {
  type PositionRepr = Span;

  fn start(&self) -> usize {
    0
  }

  fn is_eof(&self, pos: usize) -> bool {
    pos >= self.tokens.len()
  }

  fn position_repr(&self, pos: usize) -> Self::PositionRepr {
    self
      .tokens
      .get(pos)
      .map(|t| t.span)
      .unwrap_or_else(|| self.eof)
  }
}

impl<'src> peg::ParseElem<'src> for Lexer<'src> {
  type Element = &'src Token<'src>;

  fn parse_elem(&'src self, pos: usize) -> peg::RuleResult<Self::Element> {
    match self.tokens.get(pos) {
      Some(token) => peg::RuleResult::Matched(pos + 1, token),
      None => peg::RuleResult::Failed,
    }
  }
}

impl<'src> peg::ParseLiteral for Lexer<'src> {
  fn parse_string_literal(&self, pos: usize, literal: &str) -> peg::RuleResult<()> {
    let Some(token) = self.tokens.get(pos) else {
      return peg::RuleResult::Failed;
    };

    if token.lexeme.as_ref() == literal {
      peg::RuleResult::Matched(pos + 1, ())
    } else {
      peg::RuleResult::Failed
    }
  }
}

impl<'src> peg::ParseSlice<'src> for Lexer<'src> {
  type Slice = &'src [Token<'src>];

  fn parse_slice(&'src self, p1: usize, p2: usize) -> Self::Slice {
    &self.tokens[p1..p2]
  }
}

#[derive(Clone, Copy, Debug, Logos)]
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
  #[regex(r"\n\r?[ ]+")]
  _Indentation,
  #[doc(hidden)]
  #[regex(r"[ \n\r]+", logos::skip)]
  _Whitespace,
  #[doc(hidden)]
  #[regex(r"#[^\n]*", logos::skip)]
  _Comment,

  /// Errors are filtered out before parsing
  #[doc(hidden)]
  #[error]
  _Error,
}

impl<'src> fmt::Debug for Lexer<'src> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    struct DebugToken<'src>(Token<'src>);
    impl<'src> fmt::Debug for DebugToken<'src> {
      fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let TokenKind::Lit_Ident = self.0.kind {
          write!(
            f,
            "(>{} {:?} `{}` @{})",
            self.0.ws, self.0.kind, self.0.lexeme, self.0.span
          )
        } else {
          write!(f, "(>{} {:?} @{})", self.0.ws, self.0.kind, self.0.span)
        }
      }
    }

    self
      .tokens
      .clone()
      .into_iter()
      .map(DebugToken)
      .collect::<Vec<_>>()
      .fmt(f)
  }
}

#[cfg(test)]
mod tests;
