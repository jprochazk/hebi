use bumpalo::Bump;
use vm3::lex::{Lexer, Tokens};
use vm3::syn::Parser;

#[cfg(not(miri))]
#[test]
fn parser() {
  insta::glob!("input/*.h2", |path| {
    let file = std::fs::read_to_string(path).unwrap();
    let arena = Bump::new();
    let lex = Lexer::new(&file);
    let parser = Parser::new(&arena, lex);
    let snapshot = match parser.parse() {
      Ok(ast) => format!("{ast:#?}"),
      Err(e) => e,
    };
    insta::assert_snapshot!(snapshot)
  });
}

#[cfg(not(miri))]
#[test]
fn lexer() {
  insta::glob!("input/*.h2", |path| {
    let file = std::fs::read_to_string(path).unwrap();
    let tokens: Vec<_> = Tokens(Lexer::new(&file)).collect();
    let snapshot = format!("{tokens:#?}");
    insta::assert_snapshot!(snapshot)
  });
}
