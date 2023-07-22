use vm3::lex::{Lexer, Tokens};

#[cfg(any(not(miri), rust_analyzer))]
#[test]
fn lexer() {
  let mut settings = insta::Settings::clone_current();
  settings.set_snapshot_path("./lex/snapshots");
  let _scope = settings.bind_to_scope();

  insta::glob!("parse/input/*.h2", |path| {
    let file = std::fs::read_to_string(path).unwrap();
    let tokens: Vec<_> = Tokens(Lexer::new(&file)).collect();
    let snapshot = format!("{tokens:#?}");
    insta::assert_snapshot!(snapshot)
  });
}
