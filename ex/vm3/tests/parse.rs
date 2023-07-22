use bumpalo::Bump;
use vm3::lex::Lexer;
use vm3::syn::Parser;

#[cfg(any(not(miri), rust_analyzer))]
#[test]
fn parser() {
  let mut settings = insta::Settings::clone_current();
  settings.set_snapshot_path("./parse/snapshots");
  let _scope = settings.bind_to_scope();

  insta::glob!("parse/input/*.h2", |path| {
    let file = std::fs::read_to_string(path).unwrap();
    let arena = Bump::new();
    let lex = Lexer::new(&file);
    let parser = Parser::new(&arena, lex);
    let snapshot = match parser.parse() {
      Ok(ast) => format!("{ast:#?}"),
      Err(e) => format!("{e}"),
    };
    insta::assert_snapshot!(snapshot)
  });
}
