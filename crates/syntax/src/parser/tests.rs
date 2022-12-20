use diag::Source;
use indoc::indoc;
use peg::error::{ExpectedSet, ParseError};
use span::Span;

use super::*;

fn relevant_error(set: &ExpectedSet) -> Option<&str> {
  set.tokens().find(|&token| token == "invalid indentation")
}

fn report(source: &str, err: ParseError<Span>) -> String {
  let message = if let Some(e) = relevant_error(&err.expected) {
    e.to_string()
  } else {
    format!("{}", err.expected)
  };

  let report = diag::Report::error()
    .source(Source::string(source))
    .message(message)
    .span(err.location)
    .build()
    .unwrap();
  let mut buf = String::new();
  report.emit(&mut buf).unwrap();
  buf
}

fn check(input: &str) {
  let lex = Lexer::lex(input).unwrap();
  match parse(&lex) {
    Ok(module) => insta::assert_debug_snapshot!(module),
    Err(e) => {
      eprintln!("{}", report(input, e));
      panic!("Failed to parse source, see errors above.")
    }
  };
}

fn fail(input: &str) {
  let lex = Lexer::lex(input).unwrap();
  match parse(&lex) {
    Ok(_) => panic!("module parsed successfully"),
    Err(e) => insta::assert_snapshot!(report(input, e)),
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

  fail(indoc! {
    r#"
      use a
        use b
    "#
  });
}
