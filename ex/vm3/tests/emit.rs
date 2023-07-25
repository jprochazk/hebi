#[path = "./common/mod.rs"]
mod common;

use std::error::Error;

use bumpalo::Bump;
use vm3::gc::Gc;
use vm3::lex::Lexer;
use vm3::op::emit;
use vm3::syn::Parser;

#[test]
fn emit() -> Result<(), Box<dyn Error>> {
  common::snapshot(
    "emit",
    "tests/emit/input",
    "tests/emit/snapshots",
    |input| {
      let arena = Bump::new();
      let gc = Gc::new();

      let lex = Lexer::new(input);
      let parser = Parser::new(&arena, lex);
      let ast = match parser.parse() {
        Ok(ast) => ast,
        Err(e) => panic!("{e}"),
      };
      match emit::module(&arena, &gc, "test", ast) {
        Ok(module) => format!("{}", module.root().dis()),
        Err(e) => format!("{e}"),
      }
    },
  )
}

/* #[test]
fn _temp() {
  let file = "fn foo() {}";
  let arena = Bump::new();
  let lex = Lexer::new(file);
  let parser = Parser::new(&arena, lex);
  let ast = match parser.parse() {
    Ok(ast) => ast,
    Err(e) => panic!("{e}"),
  };
  let gc = Gc::new();
  let snapshot = match emit::module(&arena, &gc, "test", ast) {
    Ok(module) => format!("{}", module.root().dis()),
    Err(e) => format!("{e}"),
  };
  insta::assert_snapshot!(snapshot)
} */
