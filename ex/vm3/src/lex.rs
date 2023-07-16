use core::borrow::Borrow;
use core::fmt::Display;
use core::mem::{discriminant, swap};
use core::ops::{Index, Range};

use logos::{FilterResult, Logos};

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
      kind: TokenKind::TokEof,
      span: (end..end).into(),
    };
    let mut lex = Self {
      src,
      inner: TokenKind::lexer(src),
      previous: eof,
      current: eof,
      eof,
    };
    lex.bump();
    lex
  }

  #[inline]
  pub fn src(&self) -> &'src str {
    self.src
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
    &self.src[token.span]
  }

  #[inline]
  pub fn bump(&mut self) {
    swap(&mut self.previous, &mut self.current);
    let token = self.inner.next();
    let span: Span = self.inner.span().into();
    self.current = match token {
      Some(Ok(kind)) => Token::new(kind, span),
      Some(Err(())) => Token::new(TokenKind::TokError, span),
      None => self.eof,
    };
  }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
  pub start: u32,
  pub end: u32,
}

impl Span {
  #[inline]
  pub fn start(&self) -> usize {
    self.start as usize
  }

  #[inline]
  pub fn end(&self) -> usize {
    self.end as usize
  }

  pub fn empty() -> Span {
    Span { start: 0, end: 0 }
  }

  pub fn is_empty(&self) -> bool {
    self.start == self.end
  }
}

impl From<Range<usize>> for Span {
  #[inline]
  fn from(value: Range<usize>) -> Self {
    Span {
      start: value.start as u32,
      end: value.end as u32,
    }
  }
}

impl From<Range<u32>> for Span {
  #[inline]
  fn from(value: Range<u32>) -> Self {
    Span {
      start: value.start,
      end: value.end,
    }
  }
}

impl From<Span> for Range<usize> {
  #[inline]
  fn from(value: Span) -> Self {
    value.start as usize..value.end as usize
  }
}

impl<T: Index<Range<usize>>> Index<Span> for [T] {
  type Output = <[T] as Index<Range<usize>>>::Output;

  #[inline]
  fn index(&self, index: Span) -> &Self::Output {
    self.index(Range::from(index))
  }
}

impl Index<Span> for str {
  type Output = <str as Index<Range<usize>>>::Output;

  #[inline]
  fn index(&self, index: Span) -> &Self::Output {
    self.index(Range::from(index))
  }
}

impl Display for Span {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "{}..{}", self.start, self.end)
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
  pub kind: TokenKind,
  pub span: Span,
}

impl Token {
  pub fn new(kind: TokenKind, span: impl Into<Span>) -> Self {
    Self {
      kind,
      span: span.into(),
    }
  }

  #[inline]
  pub fn begins_expr(&self) -> bool {
    matches!(
      self.kind,
      TokenKind::KwFn
        | TokenKind::KwIf
        | TokenKind::BrkCurlyL
        | TokenKind::OpMinus
        | TokenKind::OpBang
        | TokenKind::LitNil
        | TokenKind::LitInt
        | TokenKind::LitFloat
        | TokenKind::LitBool
        | TokenKind::LitString
        | TokenKind::LitRecord
        | TokenKind::LitList
        | TokenKind::LitTuple
        | TokenKind::TokIdent
    )
  }

  #[inline]
  pub fn is(&self, kind: impl Borrow<TokenKind>) -> bool {
    discriminant(&self.kind) == discriminant(kind.borrow())
  }
}

#[derive(Clone, Copy, Debug, Logos, PartialEq, Eq)]
#[logos(skip r"[ \t\n\r]+")]
pub enum TokenKind {
  #[token("fn")]
  KwFn,
  #[token("loop")]
  KwLoop,
  #[token("break")]
  KwBreak,
  #[token("continue")]
  KwContinue,
  #[token("return")]
  KwReturn,
  #[token("if")]
  KwIf,
  #[token("else")]
  KwElse,
  #[token("let")]
  KwLet,

  // Brackets
  #[token("{")]
  BrkCurlyL,
  #[token("}")]
  BrkCurlyR,
  #[token("(")]
  BrkParenL,
  #[token(")")]
  BrkParenR,
  #[token("[")]
  BrkSquareL,
  #[token("]")]
  BrkSquareR,

  // Misc characters
  #[token(".")]
  OpDot,
  #[token(",")]
  TokComma,
  #[token(":")]
  TokColon,

  // Equals operators
  #[token("=")]
  OpEqual,
  #[token("==")]
  OpEqualEqual,
  #[token("!=")]
  OpBangEqual,
  #[token("+=")]
  OpPlusEqual,
  #[token("-=")]
  OpMinusEqual,
  #[token("/=")]
  OpSlashEqual,
  #[token("*=")]
  OpStarEqual,
  #[token("%=")]
  OpPercentEqual,
  #[token("**=")]
  OpStarStarEqual,

  // Operators
  #[token("+")]
  OpPlus,
  #[token("-")]
  OpMinus,
  #[token("/")]
  OpSlash,
  #[token("*")]
  OpStar,
  #[token("%")]
  OpPercent,
  #[token("**")]
  OpStarStar,
  #[token("!")]
  OpBang,
  #[token(">")]
  OpMore,
  #[token(">=")]
  OpMoreEqual,
  #[token("<")]
  OpLess,
  #[token("<=")]
  OpLessEqual,
  #[token("||")]
  OpPipePipe,
  #[token("&&")]
  OpAndAnd,

  /// `nil`
  #[token("nil")]
  LitNil,
  #[regex("[0-9]([0-9_]*[0-9])?", priority = 10)]
  LitInt,
  /// `0`, `1.0`, `5e10`, etc.
  #[regex(r"[0-9]+(\.[0-9]+)?([Ee][+-]?[0-9]+)?")]
  LitFloat,
  /// `true` or `false`
  #[token("true")]
  #[token("false")]
  LitBool,
  #[regex(r#""([^"\\]|\\.)*""#)] // fix highlighting -> "
  LitString,

  #[token("#{")]
  LitRecord,
  #[token("#[")]
  LitList,
  #[token("#(")]
  LitTuple,

  /// `a`, `b_c`, `__x0`, etc.
  #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
  TokIdent,

  #[regex("//[^\n]*", logos::skip)]
  TokComment,
  #[regex("/\\*", multi_line_comment)]
  TokCommentMultiLine,
  #[regex("#![^\n]*", logos::skip)]
  TokShebang,

  TokError,
  TokEof,
}

impl TokenKind {
  pub fn name(&self) -> &'static str {
    match self {
      TokenKind::KwFn => "fn",
      TokenKind::KwLoop => "loop",
      TokenKind::KwBreak => "break",
      TokenKind::KwContinue => "continue",
      TokenKind::KwReturn => "return",
      TokenKind::KwIf => "if",
      TokenKind::KwElse => "else",
      TokenKind::KwLet => "let",
      TokenKind::BrkCurlyL => "{",
      TokenKind::BrkCurlyR => "}",
      TokenKind::BrkParenL => "(",
      TokenKind::BrkParenR => ")",
      TokenKind::BrkSquareL => "[",
      TokenKind::BrkSquareR => "]",
      TokenKind::OpDot => ".",
      TokenKind::TokComma => ",",
      TokenKind::TokColon => ":",
      TokenKind::OpEqual => "=",
      TokenKind::OpEqualEqual => "==",
      TokenKind::OpBangEqual => "!=",
      TokenKind::OpPlusEqual => "==",
      TokenKind::OpMinusEqual => "-=",
      TokenKind::OpSlashEqual => "/=",
      TokenKind::OpStarEqual => "*=",
      TokenKind::OpPercentEqual => "%=",
      TokenKind::OpStarStarEqual => "**=",
      TokenKind::OpPlus => "+",
      TokenKind::OpMinus => "-",
      TokenKind::OpSlash => "/",
      TokenKind::OpStar => "*",
      TokenKind::OpPercent => "%",
      TokenKind::OpStarStar => "**",
      TokenKind::OpBang => "!",
      TokenKind::OpMore => ">",
      TokenKind::OpMoreEqual => ">=",
      TokenKind::OpLess => "<",
      TokenKind::OpLessEqual => "<=",
      TokenKind::OpPipePipe => "||",
      TokenKind::OpAndAnd => "&&",
      TokenKind::LitNil => "nil",
      TokenKind::LitInt => "int",
      TokenKind::LitFloat => "float",
      TokenKind::LitBool => "bool",
      TokenKind::LitString => "string",
      TokenKind::LitRecord => "#{",
      TokenKind::LitList => "#[",
      TokenKind::LitTuple => "#(",
      TokenKind::TokIdent => "identifier",
      TokenKind::TokComment => "comment",
      TokenKind::TokCommentMultiLine => "multi-line comment",
      TokenKind::TokShebang => "shebang",
      TokenKind::TokError => "error",
      TokenKind::TokEof => "eof",
    }
  }
}

pub struct Tokens<'src>(pub Lexer<'src>);

impl<'src> Iterator for Tokens<'src> {
  type Item = (&'src str, Token);

  fn next(&mut self) -> Option<Self::Item> {
    let token = *self.0.current();
    self.0.bump();
    if !token.is(TokenKind::TokEof) {
      Some((self.0.lexeme(&token), token))
    } else {
      None
    }
  }
}

fn multi_line_comment(lex: &mut logos::Lexer<'_, TokenKind>) -> FilterResult<(), ()> {
  // how many characters we went through
  let mut n = 0;
  // Mitigate DOS attacks on the lexer with many unclosed comments:
  //
  // /*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*/*
  //
  // Without this step, the lexer would re-attempt parsing until EOF from every
  // occurrence of /*, leading to O(N^2) worst case performance.
  let mut n_at_last_seen_opening = 0;

  // how many multi-line comment opening tokens we found
  // this starts at one, because the initial /* is already consumed
  let mut opening_count = 1;
  let mut previous_two = [b'*', b'\0'];

  for ch in lex.remainder().bytes() {
    n += 1;
    previous_two = [previous_two[1], ch];

    match previous_two {
      [b'/', b'*'] => {
        opening_count += 1;
        n_at_last_seen_opening = n
      }
      [b'*', b'/'] => opening_count -= 1,
      _ => {
        continue;
      }
    }

    if opening_count == 0 {
      break;
    }

    // Set the last byte to /0, so comments like /*/**/*/ get parsed correctly
    previous_two[1] = b'\0';
  }

  if opening_count == 0 {
    lex.bump(n);
    FilterResult::Skip
  } else {
    lex.bump(n_at_last_seen_opening);
    FilterResult::Error(())
  }
}
