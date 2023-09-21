use core::borrow::Borrow;
use core::fmt::Display;
use core::mem::{discriminant, swap};
use core::ops::{Index, Range};

use logos::{FilterResult, Logos};

#[derive(Clone)]
pub struct Lexer<'src> {
  src: &'src str,
  inner: logos::Lexer<'src, TokenKind>,
  previous: Token,
  current: Token,
}

const EOF: Token = Token {
  kind: TokenKind::Eof,
  span: Span { start: 0, end: 0 },
};

impl<'src> Lexer<'src> {
  pub fn new(src: &'src str) -> Self {
    let mut lex = Self {
      src,
      inner: TokenKind::lexer(src),
      previous: EOF,
      current: EOF,
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
      Some(Err(())) => Token::new(TokenKind::Error, span),
      None => EOF,
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

  /* #[inline]
  pub fn begins_expr(&self) -> bool {
    matches!(
      self.kind,
      TokenKind::Fn
        | TokenKind::If
        | TokenKind::CurlyL
        | TokenKind::Minus
        | TokenKind::Bang
        | TokenKind::Nil
        | TokenKind::Int
        | TokenKind::Float
        | TokenKind::Bool
        | TokenKind::String
        | TokenKind::Record
        | TokenKind::List
        | TokenKind::Tuple
        | TokenKind::Ident
    )
  } */

  #[inline]
  pub fn is(&self, kind: impl Borrow<TokenKind>) -> bool {
    discriminant(&self.kind) == discriminant(kind.borrow())
  }
}

#[derive(Clone, Copy, Debug, Logos, PartialEq, Eq)]
#[logos(skip r"[ \t\n\r]+")]
pub enum TokenKind {
  #[token("fn")]
  Fn,
  #[token("class")]
  Class,
  #[token("static")]
  Static,
  #[token("pub")]
  Pub,
  #[token("mod")]
  Mod,
  #[token("use")]
  Use,
  #[token("inter")]
  Inter,
  #[token("impl")]
  Impl,
  #[token("type")]
  Type,
  #[token("where")]
  Where,
  #[token("loop")]
  Loop,
  #[token("break")]
  Break,
  #[token("continue")]
  Continue,
  #[token("return")]
  Return,
  #[token("if")]
  If,
  #[token("else")]
  Else,
  #[token("let")]
  Let,
  #[token("while")]
  While,
  #[token("for")]
  For,
  #[token("yield")]
  Yield,
  #[token("var")]
  Var,
  #[token("union")]
  Union,
  #[token("match")]
  Match,
  #[token("case")]
  Case,

  // Brackets
  #[token("{")]
  CurlyL,
  #[token("}")]
  CurlyR,
  #[token("(")]
  ParenL,
  #[token(")")]
  ParenR,
  #[token("[")]
  SquareL,
  #[token("]")]
  SquareR,

  // Misc characters
  #[token(".")]
  Dot,
  #[token(",")]
  Comma,
  #[token(":")]
  Colon,
  #[token("->")]
  Arrow,
  #[token("@")]
  At,

  // Equals operators
  #[token("=")]
  Equal,
  #[token("==")]
  EqualEqual,
  #[token("!=")]
  BangEqual,
  #[token("+=")]
  PlusEqual,
  #[token("-=")]
  MinusEqual,
  #[token("/=")]
  SlashEqual,
  #[token("*=")]
  StarEqual,
  #[token("%=")]
  PercentEqual,
  #[token("**=")]
  StarStarEqual,
  #[token("??=")]
  CoalesceEqual,

  // Operators
  #[token("+")]
  Plus,
  #[token("-")]
  Minus,
  #[token("/")]
  Slash,
  #[token("*")]
  Star,
  #[token("%")]
  Percent,
  #[token("**")]
  StarStar,
  #[token("!")]
  Bang,
  #[token(">")]
  More,
  #[token(">=")]
  MoreEqual,
  #[token("<")]
  Less,
  #[token("<=")]
  LessEqual,
  #[token("?")]
  Question,
  #[token("??")]
  Coalesce,
  #[token("|")]
  Pipe,
  #[token("||")]
  PipePipe,
  #[token("&")]
  And,
  #[token("&&")]
  AndAnd,

  #[token("nil")]
  Nil,
  #[regex("[0-9]([0-9_]*[0-9])?", priority = 10)]
  Int,
  /// `0`, `1.0`, `5e10`, etc.
  #[regex(r"[0-9]+(\.[0-9]+)?([Ee][+-]?[0-9]+)?")]
  Float,
  #[token("true")]
  #[token("false")]
  Bool,
  #[regex(r#""([^"\\]|\\.)*""#)]
  String,

  #[token("#{")]
  Record,
  #[token("#[")]
  List,
  #[token("#(")]
  Tuple,

  /// `a`, `b_c`, `__x0`, etc.
  #[regex("[a-zA-Z_][a-zA-Z0-9_]*")]
  Ident,

  /// `'a` ,`'b_c`, `'__x0`, etc.
  #[regex("'[a-zA-Z_][a-zA-Z0-9_]*")]
  Label,

  #[regex("//[^\n]*", logos::skip)]
  Comment,
  #[regex("/\\*", multi_line_comment)]
  CommentMultiLine,
  #[regex("#![^\n]*", logos::skip)]
  Shebang,

  Error,
  Eof,
}

macro_rules! t {
  [fn] => (TokenKind::Fn);
  [class] => (TokenKind::Class);
  [static] => (TokenKind::Static);
  [pub] => (TokenKind::Pub);
  [mod] => (TokenKind::Mod);
  [use] => (TokenKind::Use);
  [inter] => (TokenKind::Inter);
  [impl] => (TokenKind::Impl);
  [type] => (TokenKind::Type);
  [where] => (TokenKind::Where);
  [loop] => (TokenKind::Loop);
  [while] => (TokenKind::While);
  [for] => (TokenKind::For);
  [break] => (TokenKind::Break);
  [continue] => (TokenKind::Continue);
  [return] => (TokenKind::Return);
  [yield] => (TokenKind::Yield);
  [if] => (TokenKind::If);
  [else] => (TokenKind::Else);
  [let] => (TokenKind::Let);
  [var] => (TokenKind::Var);
  [union] => (TokenKind::Union);
  [match] => (TokenKind::Match);
  [case] => (TokenKind::Case);
  [,] => (TokenKind::Comma);
  [:] => (TokenKind::Colon);
  [->] => (TokenKind::Arrow);
  [@] => (TokenKind::At);
  [.] => (TokenKind::Dot);
  [=] => (TokenKind::Equal);
  [==] => (TokenKind::EqualEqual);
  [!=] => (TokenKind::BangEqual);
  [==] => (TokenKind::PlusEqual);
  [-=] => (TokenKind::MinusEqual);
  [/=] => (TokenKind::SlashEqual);
  [*=] => (TokenKind::StarEqual);
  [%=] => (TokenKind::PercentEqual);
  [**=] => (TokenKind::StarStarEqual);
  [??=] => (TokenKind::CoalesceEqual);
  [+] => (TokenKind::Plus);
  [-] => (TokenKind::Minus);
  [/] => (TokenKind::Slash);
  [*] => (TokenKind::Star);
  [%] => (TokenKind::Percent);
  [**] => (TokenKind::StarStar);
  [!] => (TokenKind::Bang);
  [>] => (TokenKind::More);
  [>=] => (TokenKind::MoreEqual);
  [<] => (TokenKind::Less);
  [<=] => (TokenKind::LessEqual);
  [?] => (TokenKind::Question);
  [??] => (TokenKind::Coalesce);
  [|] => (TokenKind::Pipe);
  [||] => (TokenKind::PipePipe);
  [&] => (TokenKind::And);
  [&&] => (TokenKind::AndAnd);
  [nil] => (TokenKind::Nil);
  [int] => (TokenKind::Int);
  [float] => (TokenKind::Float);
  [bool] => (TokenKind::Bool);
  [string] => (TokenKind::String);
  [record] => (TokenKind::Record);
  [list] => (TokenKind::List);
  [tuple] => (TokenKind::Tuple);
  [ident] => (TokenKind::Ident);
  [label] => (TokenKind::Label);
}

impl TokenKind {
  pub fn name(&self) -> &'static str {
    use TokenKind as TK;
    match self {
      TK::Fn => "fn",
      TK::Class => "class",
      TK::Static => "static",
      TK::Pub => "pub",
      TK::Mod => "mod",
      TK::Use => "use",
      TK::Inter => "inter",
      TK::Impl => "impl",
      TK::Type => "type",
      TK::Where => "where",
      TK::Loop => "loop",
      TK::While => "while",
      TK::For => "for",
      TK::Break => "break",
      TK::Continue => "continue",
      TK::Return => "return",
      TK::Yield => "yield",
      TK::If => "if",
      TK::Else => "else",
      TK::Let => "let",
      TK::Var => "var",
      TK::Union => "union",
      TK::Match => "match",
      TK::Case => "case",
      TK::CurlyL => "{",
      TK::CurlyR => "}",
      TK::ParenL => "(",
      TK::ParenR => ")",
      TK::SquareL => "[",
      TK::SquareR => "]",
      TK::Comma => ",",
      TK::Colon => ":",
      TK::Arrow => "->",
      TK::At => "@",
      TK::Dot => ".",
      TK::Equal => "=",
      TK::EqualEqual => "==",
      TK::BangEqual => "!=",
      TK::PlusEqual => "==",
      TK::MinusEqual => "-=",
      TK::SlashEqual => "/=",
      TK::StarEqual => "*=",
      TK::PercentEqual => "%=",
      TK::StarStarEqual => "**=",
      TK::CoalesceEqual => "??=",
      TK::Plus => "+",
      TK::Minus => "-",
      TK::Slash => "/",
      TK::Star => "*",
      TK::Percent => "%",
      TK::StarStar => "**",
      TK::Bang => "!",
      TK::More => ">",
      TK::MoreEqual => ">=",
      TK::Less => "<",
      TK::LessEqual => "<=",
      TK::Question => "?",
      TK::Coalesce => "??",
      TK::Pipe => "|",
      TK::PipePipe => "||",
      TK::And => "&",
      TK::AndAnd => "&&",
      TK::Nil => "nil",
      TK::Int => "int",
      TK::Float => "float",
      TK::Bool => "bool",
      TK::String => "string",
      TK::Record => "#{",
      TK::List => "#[",
      TK::Tuple => "#(",
      TK::Ident => "identifier",
      TK::Label => "label",
      TK::Comment => "comment",
      TK::CommentMultiLine => "multi-line comment",
      TK::Shebang => "shebang",
      TK::Error => "error",
      TK::Eof => "eof",
    }
  }
}

pub struct Tokens<'src>(pub Lexer<'src>);

impl<'src> Iterator for Tokens<'src> {
  type Item = (&'src str, Token);

  fn next(&mut self) -> Option<Self::Item> {
    let token = *self.0.current();
    self.0.bump();
    if !token.is(TokenKind::Eof) {
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
