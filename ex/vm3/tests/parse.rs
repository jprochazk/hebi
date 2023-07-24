#[path = "./common/mod.rs"]
mod common;

use std::error::Error;

use bumpalo::Bump;
use vm3::lex::Lexer;
use vm3::syn::Parser;

#[test]
fn parser() -> Result<(), Box<dyn Error>> {
  // skip when running under miri
  if cfg!(miri) {
    return Ok(());
  }

  common::snapshot(
    "parser",
    "tests/parse/input",
    "tests/parse/snapshots",
    |input| {
      let arena = Bump::new();
      let lex = Lexer::new(input);
      let parser = Parser::new(&arena, lex);
      match parser.parse() {
        Ok(ast) => format!("{ast:#?}"),
        Err(e) => format!("{e}"),
      }
    },
  )
}
