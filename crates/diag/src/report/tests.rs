use super::{Report, Snippet};
use crate::report::{Level, Source};

#[test]
fn snippet_single_line() {
  let src = "lorem ipsum dolor sit amet consectetur adipiscing elit";

  assert_eq!(
    Snippet::new(src, 6..17),
    Snippet {
      s: "lorem ipsum dolor sit amet consectetur adipiscing elit".into(),
      line: 1,
      count: 1,
      span: (6..17).into(),
    }
  );
}

#[test]
fn snippet_multi_line() {
  struct Case {
    src: &'static str,
    src_span: std::ops::Range<usize>,
    snippet: Snippet<'static>,
  }

  let tests = vec![
    Case {
      src: "lorem ipsum\ndolor sit amet\nconsectetur adipiscing elit",
      src_span: 6..17,
      snippet: Snippet {
        s: "lorem ipsum\ndolor sit amet".into(),
        line: 1,
        count: 2,
        span: (6..17).into(),
      },
    },
    Case {
      src: "lorem ipsum\ndolor sit amet\nconsectetur adipiscing elit",
      src_span: 17..31,
      snippet: Snippet {
        s: "dolor sit amet\nconsectetur adipiscing elit".into(),
        line: 2,
        count: 2,
        span: (5..19).into(),
      },
    },
    Case {
      src: "\n\\n",
      src_span: 1..3,
      snippet: Snippet {
        s: "\\n".into(),
        line: 2,
        count: 1,
        span: (0..2).into(),
      },
    },
    Case {
      src: "d(                 ",
      src_span: 19..19,
      snippet: Snippet {
        s: "d(".into(),
        line: 1,
        count: 1,
        span: (2..2).into(),
      },
    },
    Case {
      src: "\u{9389a}\"\n",
      src_span: 4..6,
      snippet: Snippet {
        s: "\u{9389a}\"".into(),
        line: 1,
        count: 1,
        span: (4..5).into(),
      },
    },
    Case {
      src: "x ",
      src_span: 0..2,
      snippet: Snippet {
        s: "x".into(),
        line: 1,
        count: 1,
        span: (0..1).into(),
      },
    },
    Case {
      src: "З  ",
      src_span: 0..2,
      snippet: Snippet {
        s: "З".into(),
        line: 1,
        count: 1,
        span: (0..2).into(),
      },
    },
    Case {
      src: "\"\n\\",
      src_span: 0..2,
      snippet: Snippet {
        s: "\"\n\\".into(),
        line: 1,
        count: 2,
        span: (0..1).into(),
      },
    },
  ];

  for (i, case) in tests.iter().enumerate() {
    let snippet = Snippet::new(case.src, case.src_span.clone());
    assert_eq!(snippet, case.snippet, "[Test #{i}] Snippets mismatch");
    assert_eq!(
      case.src[case.src_span.start..case.src_span.end]
        .trim_end_matches('\n')
        .trim_end_matches(' '),
      snippet.highlight(),
      "[Test #{i}] Highlighted slices mismatch"
    );
  }
}

#[test]
fn emit_report_single_line() {
  let report = Report {
    level: Level::Error,
    source: Source::file("test.mu", "let x = 10\nlet y = 20;"),
    message: "expected semicolon".into(),
    span: (10..11).into(),
    label: None,
    color: true,
  };
  insta::assert_snapshot!(report.emit_to_string().unwrap());
}

#[test]
fn emit_report_multi_line() {
  let report = Report {
    level: Level::Error,
    source: Source::file("test.mu", "let x: Foo = Bar {\n  a: 0,\n  b: 0,\n};"),
    message: "mismatched type".into(),
    span: (13..36).into(),
    label: Some("expected `Foo`, found `Bar`".into()),
    color: true,
  };
  insta::assert_snapshot!(report.emit_to_string().unwrap());
}

#[test]
fn emit_report_multi_line_large() {
  let report = Report {
    level: Level::Error,
    source: Source::file(
      "test",
      "let x: Foo = Bar {\n  a: 0,\n  b: 0,\n  c: 0,\n  d: 0,\n  e: 0,\n  f: 0,\n  g: 0,\n};",
    ),
    message: "mismatched type".into(),
    span: (13..76).into(),
    label: Some("expected `Foo`, found `Bar`".into()),
    color: true,
  };
  insta::assert_snapshot!(report.emit_to_string().unwrap());
}

#[test]
fn emit_report_single_line_no_color() {
  let report = Report {
    level: Level::Error,
    source: Source::file("test.mu", "let x = 10\nlet y = 20;"),
    message: "expected semicolon".into(),
    span: (10..11).into(),
    label: None,
    color: false,
  };
  insta::assert_snapshot!(report.emit_to_string().unwrap());
}

#[test]
fn emit_report_multi_line_no_color() {
  let report = Report {
    level: Level::Error,
    source: Source::file("test.mu", "let x: Foo = Bar {\n  a: 0,\n  b: 0,\n};"),
    message: "mismatched type".into(),
    span: (13..36).into(),
    label: Some("expected `Foo`, found `Bar`".into()),
    color: false,
  };
  insta::assert_snapshot!(report.emit_to_string().unwrap());
}

#[test]
fn emit_report_multi_line_large_no_color() {
  let report = Report {
    level: Level::Error,
    source: Source::file(
      "test.mu",
      "let x: Foo = Bar {\n  a: 0,\n  b: 0,\n  c: 0,\n  d: 0,\n  e: 0,\n  f: 0,\n  g: 0,\n};",
    ),
    message: "mismatched type".into(),
    span: (13..76).into(),
    label: Some("expected `Foo`, found `Bar`".into()),
    color: false,
  };
  insta::assert_snapshot!(report.emit_to_string().unwrap());
}

#[test]
fn emit_report_multi_line_edge_case_sandwiched_newline() {
  let report = Report {
    level: Level::Error,
    source: Source::file("test.mu", "\"\n\\"),
    message: "invalid character sequence".into(),
    span: (0..2).into(),
    label: None,
    color: false,
  };
  insta::assert_snapshot!(report.emit_to_string().unwrap());
}

#[test]
fn emit_report_multi_line_edge_case_sandwiched_newline_2() {
  let report = Report {
    level: Level::Error,
    source: Source::file("test.mu", "\0\"\nl\n\n\n\n\\"),
    message: "invalid character sequence".into(),
    span: (1..8).into(),
    label: None,
    color: false,
  };
  insta::assert_snapshot!(report.emit_to_string().unwrap());
}
