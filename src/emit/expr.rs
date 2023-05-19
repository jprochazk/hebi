use super::*;
use crate::value::constant::NonNaNFloat;

impl<'src> State<'src> {
  pub fn emit_expr(&mut self, expr: &'src ast::Expr<'src>) {
    match &**expr {
      ast::ExprKind::Literal(v) => self.emit_literal_expr(v, expr.span),
      ast::ExprKind::Binary(v) => self.emit_binary_expr(v, expr.span),
      ast::ExprKind::Unary(v) => self.emit_unary_expr(v, expr.span),
      ast::ExprKind::GetVar(v) => self.emit_get_var_expr(v, expr.span),
      ast::ExprKind::SetVar(v) => self.emit_set_var_expr(v, expr.span),
      ast::ExprKind::GetField(v) => self.emit_get_field_expr(v, expr.span),
      ast::ExprKind::SetField(v) => self.emit_set_field_expr(v, expr.span),
      ast::ExprKind::GetIndex(v) => self.emit_get_index_expr(v, expr.span),
      ast::ExprKind::SetIndex(v) => self.emit_set_index_expr(v, expr.span),
      ast::ExprKind::Call(v) => self.emit_call_expr(v, expr.span),
      ast::ExprKind::GetSelf => self.emit_get_self_expr(expr.span),
      ast::ExprKind::GetSuper => self.emit_get_super_expr(expr.span),
    }
  }

  fn emit_literal_expr(&mut self, expr: &'src ast::Literal<'src>, span: Span) {
    match expr {
      ast::Literal::None => self.builder().emit(LoadNone, span),
      ast::Literal::Int(v) => self.builder().emit(LoadSmi { value: op::Smi(*v) }, span),
      ast::Literal::Float(v) => {
        // float is 4 bits so cannot be stored inline,
        // but it is interned
        let num = self.constant_value(NonNaNFloat::try_from(*v).unwrap());
        self.builder().emit(LoadConst { idx: num }, span);
      }
      ast::Literal::Bool(v) => match v {
        true => self.builder().emit(LoadTrue, span),
        false => self.builder().emit(LoadFalse, span),
      },
      ast::Literal::String(v) => {
        // `const_` interns the string
        let str = self.constant_name(v);
        self.builder().emit(LoadConst { idx: str }, span);
      }
      ast::Literal::List(list) => {
        if list.is_empty() {
          self.builder().emit(MakeListEmpty, span);
          return;
        }

        let items = self.alloc_register_slice(list.len());

        for (i, value) in list.iter().enumerate() {
          self.emit_expr(value);
          self.emit_store(items.get(i), value.span);
        }
        self.builder().emit(
          MakeList {
            start: items.access(0),
            count: op::Count(list.len() as u32),
          },
          span,
        );
        /* for register in registers.iter().rev() {
          register.access();
        } */
      }
      ast::Literal::Table(table) => {
        if table.is_empty() {
          self.builder().emit(MakeTableEmpty, span);
          return;
        }

        // TODO: from descriptor
        let pairs = self.alloc_register_slice(table.len() * 2);

        for (i, (key, value)) in table.iter().enumerate() {
          self.emit_expr(key);
          self.emit_store(pairs.get(i * 2), key.span);
          self.emit_expr(value);
          self.emit_store(pairs.get(i * 2 + 1), value.span);
        }
        self.builder().emit(
          MakeTable {
            start: pairs.access(0),
            count: op::Count(table.len() as u32),
          },
          span,
        );
        /* for (key, value) in registers.iter().rev() {
          value.access();
          key.access();
        } */
      }
    }
  }

  fn emit_binary_expr(&mut self, expr: &'src ast::Binary<'src>, span: Span) {
    // binary expressions store lhs in a register,
    // and rhs in the accumulator

    match expr.op {
      ast::BinaryOp::And | ast::BinaryOp::Or | ast::BinaryOp::Maybe => {
        return self.emit_logical_expr(expr, span)
      }
      _ => {}
    }

    let lhs = self.alloc_register();
    self.emit_expr(&expr.left);
    self.emit_store(lhs.clone(), expr.left.span);
    self.emit_expr(&expr.right);

    let lhs = lhs.access();
    match expr.op {
      ast::BinaryOp::Add => self.builder().emit(Add { lhs }, span),
      ast::BinaryOp::Sub => self.builder().emit(Sub { lhs }, span),
      ast::BinaryOp::Div => self.builder().emit(Div { lhs }, span),
      ast::BinaryOp::Mul => self.builder().emit(Mul { lhs }, span),
      ast::BinaryOp::Rem => self.builder().emit(Rem { lhs }, span),
      ast::BinaryOp::Pow => self.builder().emit(Pow { lhs }, span),
      ast::BinaryOp::Eq => self.builder().emit(CmpEq { lhs }, span),
      ast::BinaryOp::Neq => self.builder().emit(CmpNe { lhs }, span),
      ast::BinaryOp::More => self.builder().emit(CmpGt { lhs }, span),
      ast::BinaryOp::MoreEq => self.builder().emit(CmpGe { lhs }, span),
      ast::BinaryOp::Less => self.builder().emit(CmpLt { lhs }, span),
      ast::BinaryOp::LessEq => self.builder().emit(CmpLe { lhs }, span),
      ast::BinaryOp::And | ast::BinaryOp::Or | ast::BinaryOp::Maybe => unreachable!(),
    }
  }

  fn emit_logical_expr(&mut self, expr: &'src ast::Binary<'src>, span: Span) {
    match expr.op {
      ast::BinaryOp::And => {
        /*
          <left> && <right>
          v = <left>
          if v:
            v = <right>
        */
        let end = self.builder().label("end");
        self.emit_expr(&expr.left);
        self.builder().emit_jump_if_false(&end, span);
        self.emit_expr(&expr.right);
        self.builder().bind_label(end);
      }
      ast::BinaryOp::Or => {
        /*
          <left> || <right>
          v = <left>
          if !v:
            v = <right>
        */
        let rhs = self.builder().label("rhs");
        let end = self.builder().label("end");
        self.emit_expr(&expr.left);
        self.builder().emit_jump_if_false(&rhs, span);
        self.builder().emit_jump(&end, span);
        self.builder().bind_label(rhs);
        self.emit_expr(&expr.right);
        self.builder().bind_label(end);
      }
      ast::BinaryOp::Maybe => {
        /*
          <left> ?? <right>
          v = <left>
          if v is none:
            v = <right>
        */
        let use_lhs = self.builder().label("lhs");
        let end = self.builder().label("end");
        let lhs = self.alloc_register();
        self.emit_expr(&expr.left);
        self.emit_store(lhs.clone(), expr.left.span);
        self.builder().emit(IsNone, span);
        self.builder().emit_jump_if_false(&use_lhs, span);
        self.emit_expr(&expr.right);
        self.builder().emit_jump(&end, span);
        self.builder().bind_label(use_lhs);
        self.emit_load(lhs, span);
        self.builder().bind_label(end);
      }
      _ => unreachable!("not a logical expr: {:?}", expr.op),
    }
  }

  fn emit_unary_expr(&mut self, expr: &'src ast::Unary<'src>, span: Span) {
    // unary expressions only use the accumulator

    if matches!(expr.op, ast::UnaryOp::Opt) {
      return self.emit_opt_expr(expr);
    }

    self.emit_expr(&expr.right);

    match expr.op {
      ast::UnaryOp::Plus => {}
      ast::UnaryOp::Minus => self.builder().emit(Inv, span),
      ast::UnaryOp::Not => self.builder().emit(Not, span),
      ast::UnaryOp::Opt => unreachable!(),
    }
  }

  fn emit_opt_expr(&mut self, expr: &'src ast::Unary<'src>) {
    assert!(matches!(expr.op, ast::UnaryOp::Opt));

    // - emit_call_expr <- with receiver, `CallMethodOpt` or similar

    let prev = std::mem::replace(&mut self.current_function().is_in_opt_expr, true);
    self.emit_expr(&expr.right);
    let _ = std::mem::replace(&mut self.current_function().is_in_opt_expr, prev);
  }

  fn emit_get_var_expr(&mut self, expr: &'src ast::GetVar<'src>, span: Span) {
    self.emit_get(expr.name.lexeme(), span);
  }

  fn emit_set_var_expr(&mut self, expr: &'src ast::SetVar<'src>, span: Span) {
    self.emit_expr(&expr.value);
    match self.resolve_var(expr.target.name.lexeme()) {
      Get::Local(reg) => self.builder().emit(Store { reg: reg.access() }, span),
      Get::Upvalue(idx) => self.builder().emit(StoreUpvalue { idx }, span),
      Get::ModuleVar(idx) => self.builder().emit(StoreModuleVar { idx }, span),
      Get::Global => {
        let name = self.constant_name(&expr.target.name);
        self.builder().emit(StoreGlobal { name }, span);
      }
    }
  }

  fn emit_get_field_expr(&mut self, expr: &'src ast::GetField<'src>, span: Span) {
    let name = self.constant_name(&expr.name);
    self.emit_expr(&expr.target);
    if self.current_function().is_in_opt_expr {
      self.builder().emit(LoadFieldOpt { name }, span);
    } else {
      self.builder().emit(LoadField { name }, span);
    }
  }

  fn emit_set_field_expr(&mut self, expr: &'src ast::SetField<'src>, span: Span) {
    let obj = self.alloc_register();
    let get = &expr.target;
    let name = self.constant_name(&get.name);
    self.emit_expr(&get.target);
    self.emit_store(obj.clone(), get.target.span);
    self.emit_expr(&expr.value);
    self.builder().emit(
      StoreField {
        obj: obj.access(),
        name,
      },
      span,
    );
  }

  fn emit_get_index_expr(&mut self, expr: &'src ast::GetIndex<'src>, span: Span) {
    let obj = self.alloc_register();
    self.emit_expr(&expr.target);
    self.emit_store(obj.clone(), expr.target.span);
    self.emit_expr(&expr.key);
    if self.current_function().is_in_opt_expr {
      self
        .builder()
        .emit(LoadIndexOpt { obj: obj.access() }, span);
    } else {
      self.builder().emit(LoadIndex { obj: obj.access() }, span);
    }
  }

  fn emit_set_index_expr(&mut self, expr: &'src ast::SetIndex<'src>, span: Span) {
    let get = &expr.target;
    let obj = self.alloc_register();
    let key = self.alloc_register();
    self.emit_expr(&get.target);
    self.emit_store(obj.clone(), get.target.span);
    self.emit_expr(&get.key);
    self.emit_store(key.clone(), get.key.span);
    self.emit_expr(&expr.value);
    self.builder().emit(
      StoreIndex {
        obj: obj.access(),
        key: key.access(),
      },
      span,
    );
  }

  fn emit_call_expr(&mut self, expr: &'src ast::Call<'src>, span: Span) {
    self.emit_expr(&expr.target);
    if expr.args.is_empty() {
      self.builder().emit(Call0, span);
    } else {
      let args = self.alloc_register_slice(1 + expr.args.len());
      let callee = args.get(0);
      self.emit_store(callee.clone(), expr.target.span);
      for (i, value) in expr.args.iter().enumerate() {
        self.emit_expr(value);
        self.emit_store(args.get(1 + i), value.span);
      }

      self.builder().emit(
        Call {
          callee: callee.access(),
          args: op::Count(expr.args.len() as u32),
        },
        span,
      );
    }
  }

  fn emit_get_self_expr(&mut self, span: Span) {
    self.builder().emit(LoadSelf, span);
  }

  fn emit_get_super_expr(&mut self, span: Span) {
    self.builder().emit(LoadSuper, span);
  }
}
