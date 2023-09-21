use crate::ast::*;
use crate::lex::Lexer;

pub struct Parser<'src> {
  lex: Lexer<'src>,
}

impl<'src> Parser<'src> {
  pub fn new(src: &'src str) -> Parser<'src> {
    Parser {
      lex: Lexer::new(src),
    }
  }

  pub fn parse(self) -> SyntaxTree<'src> {
    todo!()
  }
}
