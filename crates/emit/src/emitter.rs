use std::ops::Deref;

use beef::lean::Cow;
use op::instruction::*;
use syntax::ast;
use value::Value;

use crate::regalloc::RegAlloc;
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

  fn b(&mut self) -> &mut Builder<Value> {
    &mut self.state.builder
  }

  fn r(&mut self) -> &mut RegAlloc {
    &mut self.state.regalloc
  }
}

struct State<'src> {
  builder: Builder<Value>,
  name: Cow<'src, str>,
  parent: Option<Box<State<'src>>>,
  regalloc: RegAlloc,
}

impl<'src> State<'src> {
  fn new(name: impl Into<Cow<'src, str>>, parent: Option<Box<State<'src>>>) -> Self {
    let name = name.into();
    Self {
      builder: Builder::new(name.to_string()),
      name,
      parent,
      regalloc: RegAlloc::new(),
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

    fn emit_var_stmt(&mut self, stmt: &ast::Var) -> Result<()> {
      todo!()
    }

    fn emit_if_stmt(&mut self, stmt: &ast::If) -> Result<()> {
      todo!()
    }

    fn emit_loop_stmt(&mut self, stmt: &ast::Loop) -> Result<()> {
      todo!()
    }

    fn emit_ctrl_stmt(&mut self, stmt: &ast::Ctrl) -> Result<()> {
      todo!()
    }

    fn emit_func_stmt(&mut self, stmt: &ast::Func) -> Result<()> {
      todo!()
    }

    fn emit_class_stmt(&mut self, stmt: &ast::Class) -> Result<()> {
      todo!()
    }

    fn emit_expr_stmt(&mut self, expr: &ast::Expr) -> Result<()> {
      self.emit_expr(expr)
    }

    fn emit_pass_stmt(&mut self) -> Result<()> {
      Ok(())
    }

    fn emit_print_stmt(&mut self, stmt: &ast::Print) -> Result<()> {
      // #for n=1
      //   emit_expr(values[0])
      //   op(Print) // prints accumulator
      // #for n>1
      //   temp = alloc_temp_register()
      //   op(CreateEmptyList, capacity=values.len())
      //   op(StoreReg, temp)
      //   #each value in values
      //     emit_expr(value)
      //     list_push(temp)
      //   op(PrintList, temp)

      todo!()
    }
  }
}

mod expr {
  use super::*;

  impl<'src> Emitter<'src> {
    /// Emit a single expression.
    ///
    /// Expressions may allocate temporary registers, but the result is always
    /// stored in the accumulator.
    pub(crate) fn emit_expr(&mut self, expr: &ast::Expr) -> Result<()> {
      match expr.deref() {
        ast::ExprKind::Literal(v) => self.emit_literal_expr(v),
        ast::ExprKind::Binary(v) => self.emit_binary_expr(v),
        ast::ExprKind::Unary(v) => self.emit_unary_expr(v),
        ast::ExprKind::GetVar(v) => self.emit_get_var_expr(v),
        ast::ExprKind::SetVar(v) => self.emit_set_var_expr(v),
        ast::ExprKind::GetField(v) => self.emit_get_field_expr(v),
        ast::ExprKind::SetField(v) => self.emit_set_field_expr(v),
        ast::ExprKind::Yield(v) => self.emit_yield_expr(v),
        ast::ExprKind::Call(v) => self.emit_call_expr(v),
      }
    }

    fn emit_literal_expr(&mut self, expr: &ast::Literal) -> Result<()> {
      todo!()
      /* match expr {
        ast::Literal::None => {
          self.b().op(PushNone);
        }
        ast::Literal::Float(v) => {
          todo!()
        }
        ast::Literal::Bool(v) => {
          self.b().op(PushBool, v);
        }
        ast::Literal::String(v) => {
          let str = self.b().constant(v);
          self.b().op(LoadConst, str);
        }
        ast::Literal::Array(list) => {
          // a := None     // a: r0
          // a = [0, 1, 2] // list: r1
          // a = [0, 1, 2] // list: r1 <-- re-used register

          // TODO: from descriptor
          let temp = self.r().temp();
          self.b().op(CreateEmptyList, ());
          self.b().op(StoreReg, temp);
          for v in list {
            self.emit_expr(v)?;
            self.b().op(ListPush, temp);
          }
          self.b().op(LoadReg, temp);
          // `temp` should end here
        }
        ast::Literal::Object(_) => todo!(),
      }
      Ok(()) */
    }

    fn emit_binary_expr(&mut self, expr: &ast::Binary) -> Result<()> {
      todo!()
    }

    fn emit_unary_expr(&mut self, expr: &ast::Unary) -> Result<()> {
      todo!()
    }

    fn emit_get_var_expr(&mut self, expr: &ast::GetVar) -> Result<()> {
      todo!()
    }

    fn emit_set_var_expr(&mut self, expr: &ast::SetVar) -> Result<()> {
      todo!()
    }

    fn emit_get_field_expr(&mut self, expr: &ast::GetField) -> Result<()> {
      todo!()
    }

    fn emit_set_field_expr(&mut self, expr: &ast::SetField) -> Result<()> {
      todo!()
    }

    fn emit_yield_expr(&mut self, expr: &ast::Yield) -> Result<()> {
      todo!()
    }

    fn emit_call_expr(&mut self, expr: &ast::Call) -> Result<()> {
      todo!()
    }
  }
}
