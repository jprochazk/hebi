use crate::error::Error;
use crate::error::Result;
use core::borrow::Borrow;
use core::fmt::Display;
use core::ops::{Index, Range};
use logos::{FilterResult, Logos};
use std::ops::Deref;

pub struct Lexer<'src> {
  src: &'src str,
  inner: logos::SpannedIter<'src, TokenKind>,
}

impl<'src> Clone for Lexer<'src> {
  fn clone(&self) -> Self {
    Self {
      src: self.src.clone(),
      inner: self.inner.deref().clone().spanned(),
    }
  }
}

pub const EOF: Token = Token {
  kind: TokenKind::Eof,
  span: Span { start: 0, end: 0 },
};

impl<'src> Lexer<'src> {
  pub fn new(src: &'src str) -> Self {
    Self {
      src,
      inner: TokenKind::lexer(src).spanned(),
    }
  }

  #[inline]
  pub fn src(&self) -> &'src str {
    self.src
  }

  #[inline]
  pub fn lexeme(&self, token: &Token) -> &'src str {
    &self.src[token.span]
  }

  #[inline]
  pub fn bump(&mut self) -> Result<Token> {
    match self.inner.next() {
      Some((Ok(tok), span)) => Ok(Token::new(tok, span)),
      Some((Err(_), span)) => Err(err!(@span, UnexpectedToken)),
      None => Ok(EOF),
    }
  }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
  pub start: u32,
  pub end: u32,
}

impl Span {
  #[inline]
  pub fn start(self) -> usize {
    self.start as usize
  }

  #[inline]
  pub fn end(self) -> usize {
    self.end as usize
  }

  #[inline]
  pub fn empty() -> Span {
    Span { start: 0, end: 0 }
  }

  #[inline]
  pub fn is_empty(self) -> bool {
    self.start == self.end
  }

  #[inline]
  pub fn to(self, other: Span) -> Span {
    Span {
      start: self.start,
      end: other.end,
    }
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
    use std::mem::discriminant;

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
  #[token("record")]
  Record,
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
  #[token("while")]
  While,
  #[token("for")]
  For,
  #[token("break")]
  Break,
  #[token("continue")]
  Continue,
  #[token("return")]
  Return,
  #[token("yield")]
  Yield,
  #[token("if")]
  If,
  #[token("else")]
  Else,
  #[token("let")]
  Let,
  #[token("var")]
  Var,
  #[token("union")]
  Union,
  #[token("enum")]
  Enum,
  #[token("match")]
  Match,
  #[token("case")]
  Case,
  #[token("do")]
  Do,

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
  #[token("_")]
  Under,
  #[token("?")]
  Qmark,
  #[token(";")]
  Semi,

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
  #[token("none")]
  None,
  #[regex(r#""([^"\\]|\\.)*""#)]
  String,

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

  Eof,
}

// TODO: wrapper macro for at!, eat!, must!, must2!
macro_rules! t {
  [fn] => ($crate::lex::TokenKind::Fn);
  [class] => ($crate::lex::TokenKind::Class);
  [static] => ($crate::lex::TokenKind::Static);
  [pub] => ($crate::lex::TokenKind::Pub);
  [mod] => ($crate::lex::TokenKind::Mod);
  [use] => ($crate::lex::TokenKind::Use);
  [inter] => ($crate::lex::TokenKind::Inter);
  [impl] => ($crate::lex::TokenKind::Impl);
  [type] => ($crate::lex::TokenKind::Type);
  [where] => ($crate::lex::TokenKind::Where);
  [loop] => ($crate::lex::TokenKind::Loop);
  [while] => ($crate::lex::TokenKind::While);
  [for] => ($crate::lex::TokenKind::For);
  [break] => ($crate::lex::TokenKind::Break);
  [continue] => ($crate::lex::TokenKind::Continue);
  [return] => ($crate::lex::TokenKind::Return);
  [yield] => ($crate::lex::TokenKind::Yield);
  [if] => ($crate::lex::TokenKind::If);
  [else] => ($crate::lex::TokenKind::Else);
  [let] => ($crate::lex::TokenKind::Let);
  [var] => ($crate::lex::TokenKind::Var);
  [union] => ($crate::lex::TokenKind::Union);
  [match] => ($crate::lex::TokenKind::Match);
  [case] => ($crate::lex::TokenKind::Case);
  [do] => ($crate::lex::TokenKind::Do);
  [,] => ($crate::lex::TokenKind::Comma);
  [:] => ($crate::lex::TokenKind::Colon);
  [->] => ($crate::lex::TokenKind::Arrow);
  [@] => ($crate::lex::TokenKind::At);
  [_] => ($crate::lex::TokenKind::Under);
  [;] => ($crate::lex::TokenKind::Semi);
  [.] => ($crate::lex::TokenKind::Dot);
  [=] => ($crate::lex::TokenKind::Equal);
  [==] => ($crate::lex::TokenKind::EqualEqual);
  [!=] => ($crate::lex::TokenKind::BangEqual);
  [==] => ($crate::lex::TokenKind::PlusEqual);
  [-=] => ($crate::lex::TokenKind::MinusEqual);
  [/=] => ($crate::lex::TokenKind::SlashEqual);
  [*=] => ($crate::lex::TokenKind::StarEqual);
  [%=] => ($crate::lex::TokenKind::PercentEqual);
  [**=] => ($crate::lex::TokenKind::StarStarEqual);
  [??=] => ($crate::lex::TokenKind::CoalesceEqual);
  [+] => ($crate::lex::TokenKind::Plus);
  [-] => ($crate::lex::TokenKind::Minus);
  [/] => ($crate::lex::TokenKind::Slash);
  [*] => ($crate::lex::TokenKind::Star);
  [%] => ($crate::lex::TokenKind::Percent);
  [**] => ($crate::lex::TokenKind::StarStar);
  [!] => ($crate::lex::TokenKind::Bang);
  [>] => ($crate::lex::TokenKind::More);
  [>=] => ($crate::lex::TokenKind::MoreEqual);
  [<] => ($crate::lex::TokenKind::Less);
  [<=] => ($crate::lex::TokenKind::LessEqual);
  [?] => ($crate::lex::TokenKind::Qmark);
  [??] => ($crate::lex::TokenKind::Coalesce);
  [|] => ($crate::lex::TokenKind::Pipe);
  [||] => ($crate::lex::TokenKind::PipePipe);
  [&] => ($crate::lex::TokenKind::And);
  [&&] => ($crate::lex::TokenKind::AndAnd);
  ["("] => ($crate::lex::TokenKind::ParenL);
  [")"] => ($crate::lex::TokenKind::ParenR);
  ["{"] => ($crate::lex::TokenKind::CurlyL);
  ["}"] => ($crate::lex::TokenKind::CurlyR);
  ["["] => ($crate::lex::TokenKind::SquareL);
  ["]"] => ($crate::lex::TokenKind::SquareR);
  [nil] => ($crate::lex::TokenKind::Nil);
  [int] => ($crate::lex::TokenKind::Int);
  [float] => ($crate::lex::TokenKind::Float);
  [bool] => ($crate::lex::TokenKind::Bool);
  [none] => ($crate::lex::TokenKind::None);
  [str] => ($crate::lex::TokenKind::String);
  [ident] => ($crate::lex::TokenKind::Ident);
  [EOF] => ($crate::lex::TokenKind::Eof);
}

struct Strings {
  name: &'static str,
}

impl TokenKind {
  fn strings(&self) -> Strings {
    use TokenKind as T;
    macro_rules! s {
      ($literal:literal) => {
        Strings { name: $literal }
      };
    }
    match self {
      T::Fn => s!("fn"),
      T::Class => s!("class"),
      T::Record => s!("record"),
      T::Static => s!("static"),
      T::Pub => s!("pub"),
      T::Mod => s!("mod"),
      T::Use => s!("use"),
      T::Inter => s!("inter"),
      T::Impl => s!("impl"),
      T::Type => s!("type"),
      T::Where => s!("where"),
      T::Loop => s!("loop"),
      T::While => s!("while"),
      T::For => s!("for"),
      T::Break => s!("break"),
      T::Continue => s!("continue"),
      T::Return => s!("return"),
      T::Yield => s!("yield"),
      T::If => s!("if"),
      T::Else => s!("else"),
      T::Let => s!("let"),
      T::Var => s!("var"),
      T::Union => s!("union"),
      T::Enum => s!("enum"),
      T::Match => s!("match"),
      T::Case => s!("case"),
      T::Do => s!("do"),
      T::CurlyL => s!("{"),
      T::CurlyR => s!("}"),
      T::ParenL => s!("("),
      T::ParenR => s!(")"),
      T::SquareL => s!("["),
      T::SquareR => s!("]"),
      T::Comma => s!(","),
      T::Colon => s!(":"),
      T::Arrow => s!("->"),
      T::At => s!("@"),
      T::Under => s!("_"),
      T::Semi => s!(";"),
      T::Dot => s!("."),
      T::Equal => s!("="),
      T::EqualEqual => s!("=="),
      T::BangEqual => s!("!="),
      T::PlusEqual => s!("=="),
      T::MinusEqual => s!("-="),
      T::SlashEqual => s!("/="),
      T::StarEqual => s!("*="),
      T::PercentEqual => s!("%="),
      T::StarStarEqual => s!("**="),
      T::CoalesceEqual => s!("??="),
      T::Plus => s!("+"),
      T::Minus => s!("-"),
      T::Slash => s!("/"),
      T::Star => s!("*"),
      T::Percent => s!("%"),
      T::StarStar => s!("**"),
      T::Bang => s!("!"),
      T::More => s!(">"),
      T::MoreEqual => s!(">="),
      T::Less => s!("<"),
      T::LessEqual => s!("<="),
      T::Qmark => s!("?"),
      T::Coalesce => s!("??"),
      T::Pipe => s!("|"),
      T::PipePipe => s!("||"),
      T::And => s!("&"),
      T::AndAnd => s!("&&"),
      T::Nil => s!("nil"),
      T::Int => s!("int"),
      T::Float => s!("float"),
      T::Bool => s!("bool"),
      T::None => s!("none"),
      T::String => s!("string"),
      T::Ident => s!("identifier"),
      T::Label => s!("label"),
      T::Comment => s!("comment"),
      T::CommentMultiLine => s!("multi-line comment"),
      T::Shebang => s!("shebang"),
      T::Eof => s!("eof"),
    }
  }

  pub fn name(&self) -> &'static str {
    self.strings().name
  }
}

pub struct Tokens<'src>(pub Lexer<'src>);

impl<'src> Iterator for Tokens<'src> {
  type Item = Result<(&'src str, Token), Error>;

  fn next(&mut self) -> Option<Self::Item> {
    match self.0.bump() {
      Ok(token) if token.is(t!(EOF)) => None,
      Ok(token) => Some(Ok((self.0.lexeme(&token), token))),
      Err(e) => Some(Err(e)),
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

impl Display for TokenKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.name())
  }
}
