use std::ops::Deref;

use beef::lean::Cow;
use op::builder::BytecodeBuilder;
use op::chunk::Chunk;
use syntax::ast;
use value::Value;

use crate::{Error, Result};

struct Emitter<'src> {
  state: State<'src>,
  module: &'src ast::Module<'src>,
}

impl<'src> Emitter<'src> {
  fn new(name: impl Into<Cow<'src, str>>, module: &'src ast::Module<'src>) -> Self {
    Self {
      state: State::new(name, None),
      module,
    }
  }

  fn emit_chunk(&mut self, f: impl FnOnce(&mut Self) -> Result<()>) -> Result<Chunk<Value>> {
    let next = State::new(self.state.name.clone(), None);
    let parent = std::mem::replace(&mut self.state, next);
    self.state.parent = Some(Box::new(parent));

    let result = f(self);

    let parent = self
      .state
      .parent
      .take()
      .expect("`self.state.parent` was set to `None` inside of callback passed to `emit_chunk`");
    let next = std::mem::replace(&mut self.state, *parent);

    result?;

    Ok(next.builder.build())
  }
}

struct State<'src> {
  builder: BytecodeBuilder<Value>,
  name: Cow<'src, str>,
  parent: Option<Box<State<'src>>>,
}

impl<'src> State<'src> {
  fn new(name: impl Into<Cow<'src, str>>, parent: Option<Box<State<'src>>>) -> Self {
    let name = name.into();
    Self {
      builder: BytecodeBuilder::new(name.to_string()),
      name,
      parent,
    }
  }
}

mod stmt {
  use super::*;

  impl<'src> Emitter<'src> {
    pub(crate) fn emit_stmt(&mut self, stmt: &'src ast::Stmt<'src>) -> Result<()> {
      match stmt.deref() {
        ast::StmtKind::Var(v) => self.emit_var_stmt(v),
        ast::StmtKind::If(v) => self.emit_if_stmt(v),
        ast::StmtKind::Loop(v) => self.emit_loop_stmt(v),
        ast::StmtKind::Ctrl(v) => self.emit_ctrl_stmt(v),
        ast::StmtKind::Func(v) => self.emit_func_stmt(v),
        ast::StmtKind::Class(v) => self.emit_class_stmt(v),
        ast::StmtKind::Expr(v) => self.emit_expr_stmt(v),
        ast::StmtKind::Pass => self.emit_pass_stmt(),
        ast::StmtKind::Print(v) => self.emit_print_stmt(v),
      }
    }

    fn emit_var_stmt(&mut self, v: &ast::Var) -> Result<()> {
      todo!()
    }

    fn emit_if_stmt(&mut self, v: &ast::If) -> Result<()> {
      todo!()
    }

    fn emit_loop_stmt(&mut self, v: &ast::Loop) -> Result<()> {
      todo!()
    }

    fn emit_ctrl_stmt(&mut self, v: &ast::Ctrl) -> Result<()> {
      todo!()
    }

    fn emit_func_stmt(&mut self, v: &ast::Func) -> Result<()> {
      todo!()
    }

    fn emit_class_stmt(&mut self, v: &ast::Class) -> Result<()> {
      todo!()
    }

    fn emit_expr_stmt(&mut self, v: &ast::Expr) -> Result<()> {
      self.emit_expr(v)
    }

    fn emit_pass_stmt(&mut self) -> Result<()> {
      Ok(())
    }

    fn emit_print_stmt(&mut self, v: &ast::Print) -> Result<()> {
      // 1. create an empty list (reg=list)
      // 2. emit all `v.values` into the list
      // 3. emit op_print with reg=list

      todo!()
    }
  }
}

mod expr {
  use super::*;

  impl<'src> Emitter<'src> {
    pub(crate) fn emit_expr(&mut self, v: &ast::Expr) -> Result<()> {
      todo!()
    }
  }
}
