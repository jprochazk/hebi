use std::ops::Deref;

use Instruction::*;

use super::*;

impl<'cx, 'src> State<'cx, 'src> {
  pub(super) fn emit_stmt(&mut self, stmt: &'src ast::Stmt<'src>) {
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
      ast::StmtKind::Import(v) => self.emit_import_stmt(v),
    }
  }

  fn emit_var_stmt(&mut self, stmt: &'src ast::Var<'src>) {
    todo!()
  }

  fn emit_if_stmt(&mut self, stmt: &'src ast::If<'src>) {
    todo!()
  }

  fn emit_loop_stmt(&mut self, stmt: &'src ast::Loop<'src>) {
    todo!()
  }

  fn emit_ctrl_stmt(&mut self, stmt: &'src ast::Ctrl<'src>) {
    todo!()
  }

  fn emit_func_stmt(&mut self, stmt: &'src ast::Func<'src>) {
    todo!()
  }

  fn emit_class_stmt(&mut self, stmt: &'src ast::Class<'src>) {
    todo!()
  }

  fn emit_expr_stmt(&mut self, stmt: &'src ast::Expr<'src>) {
    todo!()
  }

  fn emit_pass_stmt(&mut self) {
    todo!()
  }

  fn emit_print_stmt(&mut self, stmt: &'src ast::Print<'src>) {
    todo!()
  }

  fn emit_import_stmt(&mut self, stmt: &'src ast::Import<'src>) {
    todo!()
  }
}
