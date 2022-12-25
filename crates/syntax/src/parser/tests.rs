use diag::Source;
use indoc::indoc;
use peg::error::ParseError;
use span::Span;

use super::*;

fn report(source: &str, err: ParseError<Span>) -> String {
  let message = if let Some(err) = err.expected.tokens().find(|&t| t.starts_with("@@")) {
    err.strip_prefix("@@").unwrap_or(err).to_string()
  } else {
    err.expected.to_string()
  };

  diag::Report::error()
    .source(Source::string(source))
    .message(message)
    .span(err.location)
    .build()
    .unwrap()
    .emit_to_string()
    .unwrap()
}

fn print_tokens(lex: &Lexer) {
  for token in lex.debug_tokens() {
    println!("{token:?}");
  }
  // let tokens = lex.debug_tokens().collect::<Vec<_>>();
  // insta::assert_debug_snapshot!(tokens);
}

macro_rules! check_module {
  ($input:literal) => {check_module!(__inner $input, false)};
  (? $input:literal) => {check_module!(__inner $input, true)};
  (__inner $input:literal , $print_tokens:expr) => {{
    let input = indoc!($input);
    let lex = Lexer::lex(input).unwrap();
    if $print_tokens { print_tokens(&lex); }
    match parse(&lex) {
      Ok(module) => insta::assert_debug_snapshot!(module),
      Err(e) => {
        eprintln!("{}", report(input, e));
        panic!("Failed to parse source, see errors above.")
      }
    };
  }};
}

macro_rules! check_expr {
  ($input:literal) => {check_expr!(__inner $input, false)};
  (? $input:literal) => {check_expr!(__inner $input, true)};
  (__inner $input:literal , $print_tokens:expr) => {{
    let input = indoc!($input);
    let lex = Lexer::lex(input).unwrap();
    if $print_tokens { print_tokens(&lex); }
    match grammar::expr(&lex, &StateRef::new(&lex)) {
      Ok(module) => insta::assert_debug_snapshot!(module),
      Err(e) => {
        eprintln!("{}", report(input, e));
        panic!("Failed to parse source, see errors above.")
      }
    };
  }};
}

macro_rules! check_error {
  ($input:literal) => {check_error!(__inner $input, false)};
  (? $input:literal) => {check_error!(__inner $input, true)};
  (__inner $input:literal , $print_tokens:expr) => {{
    let input = indoc!($input);
    let lex = Lexer::lex(input).unwrap();
    if $print_tokens { print_tokens(&lex); }
    match parse(&lex) {
      Ok(_) => panic!("module parsed successfully"),
      Err(e) => insta::assert_snapshot!(report(input, e)),
    };
  }};
}

#[test]
fn test_import_path() {
  check_module! {
    r#"
      use a
      use a.b
      use a.b.c
      use a.{b, c}
      use a.{b.{c}, d.{e}}
      use {a.{b}, c.{d}}
      use {a, b, c,}
    "#
  };

  check_module! {
    r#"
      use a as x
      use a.b as x
      use a.b.c as x
      use a.{b as x, c as y}
      use a.{b.{c as x}, d.{e as y}}
      use {a.{b as x}, c.{d as y}}
      use {a as x, b as y, c as z,}
    "#
  };

  check_error! {
    r#"
      use a
        use b
    "#
  };
}

#[test]
fn binary_expr() {
  check_expr!(r#"a + b"#);
  check_expr!(r#"a - b"#);
  check_expr!(r#"a / b"#);
  check_expr!(r#"a ** b"#);
  check_expr!(r#"a * b"#);
  check_expr!(r#"a % b"#);
  check_expr!(r#"a == b"#);
  check_expr!(r#"a != b"#);
  check_expr!(r#"a > b"#);
  check_expr!(r#"a >= b"#);
  check_expr!(r#"a < b"#);
  check_expr!(r#"a <= b"#);
  check_expr!(r#"a && b"#);
  check_expr!(r#"a || b"#);
  check_expr!(r#"a ?? b"#);

  check_module! {
    r#"
      a + b
      c + d
    "#
  };

  check_error! {
    r#"
      a +
        b
    "#
  }

  check_error! {
    r#"
      a
      + b
    "#
  }
}

#[test]
fn unary_expr() {
  // check_expr!(r#"+a"#);
  check_expr!(r#"-a"#);
  check_expr!(r#"!a"#);
}

#[test]
fn postfix_expr() {
  check_expr!(r#"a.b[c].d"#);
  check_module! {
    r#"
      a.b[c].d
      a.b[c].d
    "#
  };

  check_error! {
    r#"
      a
      .b[c].d
    "#
  }
  check_error! {
    r#"
      a.b[c]
      .d
    "#
  }
}

#[test]
fn call_expr() {
  check_expr!(r#"a(b, c, d=e, f=g)"#);
  check_module! {
    r#"
      a(b, c, d=e, f=g)
      a(
        b, 
      c, d
          =e, 
        f=
        g,
          )
    "#
  };

  check_error! {
    r#"
      a(b=c, d)
    "#
  }
}

#[test]
fn simple_literal_expr() {
  check_module! {
    r#"
      null
      true
      false
      1
      0.1
      1.5e3
      3.14e-3
      "\tas\\df\x2800\n"
    "#
  }
}

#[test]
fn array_literal_expr() {
  check_module! {
    r#"
      [0, 1, 2]
      [0,
       1,
       2,]
      [
        0,
        1,
        2,
      ]
    "#
  }
}

#[test]
fn object_literal_expr() {
  check_module! {
    r#"
      {a:b, c:d, e:f}
      {a:b,
        c:d,
        e:f,}
      {
        a:b,
        c:d,
        e:f,
      }
    "#
  }

  check_module! {
    r#"
      {[a]:b, [c]:d, [e]:f}
      {[a]:b,
       [c]:d,
       [e]:f,}
      {
        [a]:b,
        [c]:d,
        [e]:f,
      }
    "#
  }
}
