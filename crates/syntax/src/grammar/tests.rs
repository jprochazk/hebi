use diag::Source;
use indoc::indoc;
use peg::error::ParseError;
use span::Span;

use super::*;

fn panic_report(source: &str, err: ParseError<Span>) {
  let report = diag::Report::error()
    .source(Source::string(source))
    .message(format!("expected one of: {}", err.expected).into())
    .span(err.location)
    .build()
    .unwrap();
  let mut buf = String::new();
  report.emit(&mut buf).unwrap();
  eprintln!("{buf}");
  panic!("Failed to parse source, see errors above.")
}

fn check(input: &str) {
  let lex = Lexer::lex(input).unwrap();
  match parse(&lex) {
    Ok(module) => insta::assert_debug_snapshot!(module),
    Err(e) => panic_report(input, e),
  };
}

#[test]
fn test_import_path() {
  check(indoc! {
    r#"
      use a
      use a.b
      use a.b.c
      use a.{b, c}
      use a.{b.{c}, d.{e}}
      use {a.{b}, c.{d}}
      use {a, b, c,}
    "#
  });

  check(indoc! {
    r#"
      use a as x
      use a.b as x
      use a.b.c as x
      use a.{b as x, c as y}
      use a.{b.{c as x}, d.{e as y}}
      use {a.{b as x}, c.{d as y}}
      use {a as x, b as y, c as z,}
    "#
  });
}
