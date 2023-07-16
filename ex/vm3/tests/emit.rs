use bumpalo::Bump;
use vm3::gc::Gc;
use vm3::lex::Lexer;
use vm3::op::emit;
use vm3::syn::Parser;

#[cfg(not(miri))]
#[test]
fn emit() {
  insta::glob!("input/vars.h2", |path| {
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
