#[path = "./common/mod.rs"]
mod common;

use std::error::Error;

use vm3::lex::{Lexer, Tokens};

#[test]
fn lexer() -> Result<(), Box<dyn Error>> {
  // skip when running under miri
  if cfg!(miri) {
    return Ok(());
  }

  common::snapshot(
    "lexer",
    "tests/parse/input",
    "tests/lex/snapshots",
    |input| {
      let tokens: Vec<_> = Tokens(Lexer::new(input)).collect();
      format!("{tokens:#?}")
    },
  )
}
