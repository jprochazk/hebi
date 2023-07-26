use bumpalo::Bump;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use vm3::gc::Gc;
use vm3::lex::Lexer;
use vm3::obj::func::Function;
use vm3::obj::list::List;
use vm3::obj::module::{Module, ModuleRegistry};
use vm3::op::emit;
use vm3::syn::Parser;
use vm3::vm::Thread;

pub fn fib_15(c: &mut Criterion) {
  c.bench_function("fib(15)", |b| {
    b.iter_with_setup(
      || {
        let name = "test";
        let contents = indoc::indoc! {r#"
          {
            fn fib(n) {
              if n <= 1 { n }
              else { fib(n-2) + fib(n-1) }
            }
          
            fib(15)
          }
        "#};

        let arena = Bump::new();
        let gc = Gc::new();
        let registry = ModuleRegistry::new(&gc).unwrap();

        let lex = Lexer::new(contents);
        let parser = Parser::new(name, &arena, lex);
        let ast = match parser.parse() {
          Ok(ast) => ast,
          Err(e) => panic!("{}", e.report()),
        };
        let module = match emit::module(&arena, &gc, registry, name, ast) {
          Ok(module) => module,
          Err(e) => panic!("{}", e.report()),
        };
        let root = Function::new(&gc, module.root(), List::new(&gc).unwrap()).unwrap();
        let module = Module::new(&gc, module, Some(root)).unwrap();
        registry.try_insert(&gc, module).unwrap();

        let thread = Thread::new(&gc).unwrap();
        (gc, registry, root, thread)
      },
      |(gc, registry, root, thread)| {
        match thread.run(&gc, registry, root) {
          Ok(v) => black_box(v),
          Err(e) => panic!("{}", e.report()),
        };
      },
    )
  });
}

criterion_group!(benches, fib_15);
criterion_main!(benches);
