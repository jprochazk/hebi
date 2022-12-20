use crate::ast::Module;
use crate::lexer::Lexer;

pub struct State<'src, 'lex> {
  pub lexer: &'lex Lexer<'src>,
  pub indent: IndentStack,
  pub module: Module<'src>,
}

impl<'src, 'lex> State<'src, 'lex> {
  pub fn new(lexer: &'lex Lexer<'src>) -> Self {
    Self {
      lexer,
      indent: IndentStack::new(),
      module: Module::new(),
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

  pub fn level(&self) -> u64 {
    self.level
  }

  pub fn is_indent_eq(&self, n: u64) -> bool {
    if self.ignore {
      true
    } else {
      self.level == n
    }
  }

  pub fn is_indent_gt(&self, n: u64) -> bool {
    if self.ignore {
      true
    } else {
      self.level < n
    }
  }

  pub fn ignore(&mut self, v: bool) {
    self.ignore = v;
  }

  pub fn indent(&mut self, n: u64) {
    if self.ignore {
      return;
    }
    self.stack.push(n);
    self.level += n;
  }

  pub fn dedent(&mut self) {
    if self.ignore {
      return;
    }
    let n = self.stack.pop().unwrap();
    self.level -= n;
  }
}
