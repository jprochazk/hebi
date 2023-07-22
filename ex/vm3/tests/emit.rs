use bumpalo::Bump;
use vm3::gc::Gc;
use vm3::lex::Lexer;
use vm3::op::emit;
use vm3::syn::Parser;

#[cfg(any(not(miri), rust_analyzer))]
#[test]
fn emit() {
  let mut settings = insta::Settings::clone_current();
  settings.set_snapshot_path("./emit/snapshots");
  let _scope = settings.bind_to_scope();

  insta::glob!("emit/input/*.h2", |path| {
    let file = std::fs::read_to_string(path).unwrap();
    let arena = Bump::new();
    let lex = Lexer::new(&file);
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
  });
}

/* #[test]
fn _temp() {
  let file = include_str!("./emit/input/binary.h2");
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
