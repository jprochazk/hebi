use crate::ast::{Args, Expr, Ident, Module};
use crate::lexer::Lexer;

pub struct State<'src, 'lex> {
  pub lexer: &'lex Lexer<'src>,
  pub indent: IndentStack,
  pub module: Module<'src>,
  pub temp: Temp<'src>,
}

#[derive(Default)]
pub struct Temp<'src> {
  pub call_args: Args<'src>,
  pub array_items: Vec<Expr<'src>>,
  pub object_fields: Vec<(Ident<'src>, Expr<'src>)>,
}

impl<'src, 'lex> State<'src, 'lex> {
  pub fn new(lexer: &'lex Lexer<'src>) -> Self {
    Self {
      lexer,
      indent: IndentStack::new(),
      module: Module::new(),
      temp: Temp {
        call_args: Args::new(),
        array_items: Vec::new(),
        object_fields: Vec::new(),
      },
    }
  }
}

pub struct IndentStack {
  stack: Vec<u64>,
  level: u64,
  ignore: bool,
}

impl IndentStack {
  pub fn new() -> Self {
    Self {
      stack: vec![0],
      level: 0,
      ignore: false,
    }
  }

  pub fn is_indent_eq(&self, n: u64) -> bool {
    self.level == n
  }

  pub fn is_indent_gt(&self, n: u64) -> bool {
    self.level < n
  }

  pub fn is_indent_lt(&self, n: u64) -> bool {
    self.level > n
  }

  pub fn ignore(&mut self, v: bool) {
    self.ignore = v;
  }

  pub fn is_ignored(&self) -> bool {
    self.ignore
  }

  pub fn indent(&mut self, n: u64) {
    self.stack.push(n);
    self.level += n;
  }

  pub fn dedent(&mut self) {
    let n = self.stack.pop().unwrap();
    self.level -= n;
  }
}
