use std::cell::RefCell;

use crate::ast;
use crate::ast::{Args, Expr, Module};
use crate::lexer::{Lexer, Token};

pub struct State<'src> {
  pub lexer: &'src Lexer<'src>,
  pub indent: IndentStack,
  pub module: Module<'src>,
  pub temp: Temp<'src>,
}

pub struct Temp<'src> {
  pub call_args: Args<'src>,
  pub array_items: Vec<Expr<'src>>,
  pub object_fields: Vec<(Expr<'src>, Expr<'src>)>,
}

impl<'src> State<'src> {
  pub fn new(lexer: &'src Lexer<'src>) -> Self {
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
  ignore_depth: u32,
}

impl IndentStack {
  pub fn new() -> Self {
    Self {
      stack: vec![0],
      level: 0,
      ignore_depth: 0,
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
    if v {
      self.ignore_depth += 1
    } else {
      self.ignore_depth -= 1
    }
  }

  pub fn is_ignored(&self) -> bool {
    self.ignore_depth > 0
  }

  pub fn push_indent(&mut self, n: u64) {
    self.stack.push(n);
    self.level += n;
  }

  pub fn pop_indent(&mut self) {
    let n = self.stack.pop().unwrap();
    self.level -= n;
  }
}

pub struct StateRef<'src>(RefCell<State<'src>>);

impl<'src> StateRef<'src> {
  pub fn new(lexer: &'src Lexer<'src>) -> Self {
    Self(RefCell::new(State::new(lexer)))
  }

  pub fn push_stmt(&self, stmt: ast::Stmt<'src>) {
    self.0.borrow_mut().module.body.push(stmt);
  }

  pub fn push_import(&self, import: ast::Import<'src>) {
    self.0.borrow_mut().module.imports.push(import)
  }

  pub fn get_token(&self, pos: usize) -> &'src Token {
    self.0.borrow().lexer.get(pos).unwrap()
  }

  pub fn get_lexeme(&self, token: &'src Token) -> &'src str {
    let lexer = self.0.borrow().lexer;
    lexer.lexeme(token)
  }

  pub fn push_indent(&self, n: u64) {
    self.0.borrow_mut().indent.push_indent(n);
  }

  pub fn pop_indent(&self) {
    self.0.borrow_mut().indent.pop_indent();
  }

  pub fn ignore_indent(&self, v: bool) {
    self.0.borrow_mut().indent.ignore(v)
  }

  pub fn is_indent_ignored(&self) -> bool {
    self.0.borrow().indent.is_ignored()
  }

  pub fn is_indent_eq(&self, n: u64) -> bool {
    self.0.borrow().indent.is_indent_eq(n)
  }

  pub fn is_indent_lt(&self, n: u64) -> bool {
    self.0.borrow().indent.is_indent_lt(n)
  }

  pub fn is_indent_gt(&self, n: u64) -> bool {
    self.0.borrow().indent.is_indent_gt(n)
  }

  pub fn into_inner(self) -> State<'src> {
    self.0.into_inner()
  }

  pub fn inner(&self) -> &RefCell<State<'src>> {
    &self.0
  }
}
