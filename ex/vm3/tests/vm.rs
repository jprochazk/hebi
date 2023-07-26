#[path = "./common/mod.rs"]
mod common;

use std::error::Error;

use bumpalo::Bump;
use vm3::gc::Gc;
use vm3::lex::Lexer;
use vm3::obj::func::Function;
use vm3::obj::list::List;
use vm3::obj::module::{Module, ModuleRegistry};
use vm3::op::emit;
use vm3::syn::Parser;
use vm3::vm::Thread;

#[test]
fn emit() -> Result<(), Box<dyn Error>> {
  common::snapshot("emit", "tests/vm/input", "tests/vm/snapshots", |input| {
    let arena = Bump::new();
    let gc = Gc::new();
    let registry = ModuleRegistry::new(&gc).unwrap();

    let lex = Lexer::new(input.contents);
    let parser = Parser::new(input.name, &arena, lex);
    let ast = match parser.parse() {
      Ok(ast) => ast,
      Err(e) => panic!("{}", e.report()),
    };
    let module = match emit::module(&arena, &gc, registry, input.name, ast) {
      Ok(module) => module,
      Err(e) => panic!("{}", e.report()),
    };
    let root = Function::new(&gc, module.root(), List::new(&gc)?)?;
    let module = Module::new(&gc, module, Some(root))?;
    registry.try_insert(&gc, module)?;

    println!("{}", root.dis());
    let thread = Thread::new(&gc)?;
    match thread.run(&gc, registry, root) {
      Ok(v) => Ok(v.to_string()),
      Err(e) => Ok(e.report()),
    }
  })
}

/* #[test]
fn _temp() -> Result<(), Box<dyn Error>> {
  for _ in 0..10 {
    println!("start");
    let input = common::Input {
      name: "test",
      contents: include_str!("./vm/input/fib.h2"),
    };

    let arena = Bump::new();
    let gc = Gc::new();
    let registry = ModuleRegistry::new(&gc).unwrap();

    let lex = Lexer::new(input.contents);
    let parser = Parser::new(input.name, &arena, lex);
    let ast = match parser.parse() {
      Ok(ast) => ast,
      Err(e) => panic!("{}", e.report()),
    };
    let module = match emit::module(&arena, &gc, registry, input.name, ast) {
      Ok(module) => module,
      Err(e) => panic!("{}", e.report()),
    };
    let root = Function::new(&gc, module.root(), List::new(&gc)?)?;
    let module = Module::new(&gc, module, Some(root))?;
    registry.try_insert(&gc, module)?;

    println!("{}", root.dis());
    let thread = Thread::new(&gc)?;
    match thread.run(&gc, registry, root) {
      Ok(v) => println!("{v}"),
      Err(e) => panic!("{}", e.report()),
    };
    println!("end");
  }

  Ok(())
} */
