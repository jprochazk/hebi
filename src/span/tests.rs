use super::*;

macro_rules! check {
  ($input:literal, $span:expr) => {{
    assert_snapshot!(SpannedError::new("error: test", $span).report($input, true));
  }};
}

#[test]
fn snippet_single_line() {
  check!(
    "lorem ipsum dolor sit amet consectetur adipiscing elit",
    6..17
  );
}

#[test]
fn snippet_multi_line() {
  check!(
    "lorem ipsum\ndolor sit amet\nconsectetur adipiscing elit",
    6..17
  );
  check!(
    "lorem ipsum\ndolor sit amet\nconsectetur adipiscing elit",
    17..31
  );
  check!("\n\\n", 1..3);
  check!("d(                 ", 19..19);
  check!("\u{9389a}\"\n", 4..6);
  check!("x ", 0..2);
  check!("З  ", 0..2);
  check!("\"\n\\", 0..2);
}

#[test]
fn emit_report_single_line() {
  check!("let x = 10\nlet y = 20;", 10..11);
}

#[test]
fn emit_report_multi_line() {
  check!("let x: Foo = Bar {\n  a: 0,\n  b: 0,\n};", 13..36);
}

#[test]
fn emit_report_multi_line_large() {
  check!(
    "let x: Foo = Bar {\n  a: 0,\n  b: 0,\n  c: 0,\n  d: 0,\n  e: 0,\n  f: 0,\n  g: 0,\n};",
    13..76
  );
}

#[test]
fn emit_report_multi_line_edge_case_sandwiched_newline() {
  check!("\"\n\\", 0..2);
}

#[test]
fn emit_report_multi_line_edge_case_sandwiched_newline_2() {
  check!("\0\"\nl\n\n\n\n\\", 1..8);
}

#[allow(clippy::no_effect)]
#[test]
fn test_spanned() {
  #[derive(Default)]
  struct Nested {
    v: i32,
  }
  #[derive(Default)]
  struct Test {
    a: i32,
    b: i32,
    c: i32,
    nested: Nested,
  }

  let mut t = Spanned::new(0..10, Test::default());

  t.span.start;
  t.span.end;
  t.a;
  t.b;
  t.c;
  t.nested.v = 10;

  let _ = &"asdfasdfasdfasdfasdf"[t.span];
}
