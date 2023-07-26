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
      let lex = Lexer::new(input.contents);
      let parser = Parser::new(input.name, &arena, lex);
      match parser.parse() {
        Ok(ast) => Ok(format!("{ast:#?}")),
        Err(e) => Ok(e.report()),
      }
    },
  )
}
