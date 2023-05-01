use std::ops::Deref;

use super::*;
use crate::util::JoinIter;

impl<'cx, 'src> State<'cx, 'src> {
  pub(super) fn emit_stmt(&mut self, stmt: &'src ast::Stmt<'src>) {
    match stmt.deref() {
      ast::StmtKind::Var(v) => self.emit_var_stmt(v, stmt.span),
      ast::StmtKind::If(v) => self.emit_if_stmt(v, stmt.span),
      ast::StmtKind::Loop(v) => self.emit_loop_stmt(v, stmt.span),
      ast::StmtKind::Ctrl(v) => self.emit_ctrl_stmt(v, stmt.span),
      ast::StmtKind::Func(v) => self.emit_func_stmt(v),
      ast::StmtKind::Class(v) => self.emit_class_stmt(v),
      ast::StmtKind::Expr(v) => self.emit_expr_stmt(v),
      ast::StmtKind::Pass => self.emit_pass_stmt(),
      ast::StmtKind::Print(v) => self.emit_print_stmt(v, stmt.span),
      ast::StmtKind::Import(v) => self.emit_import_stmt(v, stmt.span),
    }
  }

  fn emit_stmt_list(&mut self, list: &'src [ast::Stmt<'src>]) {
    for stmt in list {
      self.emit_stmt(stmt)
    }
  }

  fn emit_var_stmt(&mut self, stmt: &'src ast::Var<'src>, span: Span) {
    self.emit_expr(&stmt.value);
    self.emit_var(stmt.name.lexeme(), span)
  }

  fn emit_if_stmt(&mut self, stmt: &'src ast::If<'src>, span: Span) {
    // exit label for all branches
    let end = self.builder().multi_label("end");

    for branch in stmt.branches.iter() {
      let next = self.builder().label("next");
      self.emit_expr(&branch.cond);
      self.builder().emit_jump_if_false(&next, span);
      self.current_function().enter_scope();
      for stmt in branch.body.iter() {
        self.emit_stmt(stmt);
      }
      self.builder().emit_jump(&end, span);
      self.current_function().leave_scope();
      self.builder().bind_label(next);
    }

    if let Some(default) = stmt.default.as_ref() {
      self.current_function().enter_scope();
      for stmt in default.iter() {
        self.emit_stmt(stmt);
      }
      self.current_function().leave_scope();
    }

    self.builder().bind_label(end);
  }

  fn emit_loop_stmt(&mut self, stmt: &'src ast::Loop<'src>, span: Span) {
    match stmt {
      ast::Loop::For(v) => match &v.iter {
        ast::ForIter::Range(range) => self.emit_for_range_loop(v, range),
        ast::ForIter::Expr(iter) => self.emit_for_iter_loop(v, iter),
      },
      ast::Loop::While(v) => self.emit_while_loop(v, span),
      ast::Loop::Infinite(v) => self.emit_inf_loop(v, span),
    }
  }

  fn emit_for_range_loop(&mut self, stmt: &'src ast::For<'src>, range: &'src ast::IterRange<'src>) {
    let cond = self.builder().loop_header();
    let latch = self.builder().loop_header();
    let body = self.builder().label("body");
    let end = self.builder().multi_label("end");

    self.current_function().enter_scope();

    let item_value = self.alloc_register();
    let end_value = self.alloc_register();

    self.declare_local(stmt.item.lexeme(), item_value.clone());
    self.emit_expr(&range.start);
    self.builder().emit(
      Store {
        reg: item_value.access(),
      },
      stmt.item.span,
    );

    self.emit_expr(&range.end);
    self.builder().emit(
      Store {
        reg: end_value.access(),
      },
      range.span(),
    );

    self.builder().bind_loop_header(&cond);
    self.builder().emit(
      Load {
        reg: end_value.access(),
      },
      range.span(),
    );
    if range.inclusive {
      self.builder().emit(
        CmpLe {
          lhs: item_value.access(),
        },
        range.span(),
      );
    } else {
      self.builder().emit(
        CmpLt {
          lhs: item_value.access(),
        },
        range.span(),
      );
    }
    self.builder().emit_jump_if_false(&end, range.span());
    self.builder().emit_jump(&body, range.span());

    self.builder().bind_loop_header(&latch);
    self
      .builder()
      .emit(LoadSmi { value: op::Smi(1) }, range.span());
    self.builder().emit(
      Add {
        lhs: item_value.access(),
      },
      range.span(),
    );
    self.builder().emit(
      Store {
        reg: item_value.access(),
      },
      range.span(),
    );
    self.builder().emit_jump_loop(&cond, range.span());

    self.builder().bind_label(body);
    let (latch, end) = self.emit_loop_body((latch, end), &stmt.body);
    self.builder().emit_jump_loop(&latch, range.span());

    end_value.access();
    item_value.access();

    self.builder().bind_label(end);
    self.current_function().leave_scope();
  }

  fn emit_for_iter_loop(&mut self, _: &'src ast::For<'src>, _: &'src ast::Expr<'src>) {
    todo!()
  }

  fn emit_while_loop(&mut self, stmt: &'src ast::While<'src>, span: Span) {
    let start = self.builder().loop_header();
    let end = self.builder().multi_label("end");

    self.current_function().enter_scope();
    self.builder().bind_loop_header(&start);

    self.emit_expr(&stmt.cond);
    self.builder().emit_jump_if_false(&end, stmt.cond.span);

    let (start, end) = self.emit_loop_body((start, end), &stmt.body);
    self.builder().emit_jump_loop(&start, span);

    self.builder().bind_label(end);
    self.current_function().leave_scope();
  }

  fn emit_inf_loop(&mut self, stmt: &'src ast::Infinite<'src>, span: Span) {
    let start = self.builder().loop_header();
    let end = self.builder().multi_label("end");

    self.current_function().enter_scope();
    self.builder().bind_loop_header(&start);

    let (start, end) = self.emit_loop_body((start, end), &stmt.body);
    self.builder().emit_jump_loop(&start, span);

    self.builder().bind_label(end);
    self.current_function().leave_scope();
  }

  fn emit_loop_body(
    &mut self,
    (start, end): (LoopHeader, MultiLabel),
    body: &'src [ast::Stmt<'src>],
  ) -> (LoopHeader, MultiLabel) {
    let previous = self.current_function().enter_loop_body(start, end);
    self.emit_stmt_list(body);
    let current = self.current_function().leave_loop_body(previous);
    (current.start, current.end)
  }

  fn emit_ctrl_stmt(&mut self, stmt: &'src ast::Ctrl<'src>, span: Span) {
    match stmt {
      ast::Ctrl::Return(stmt) => {
        if let Some(value) = stmt.value.as_ref() {
          self.emit_expr(value);
        } else {
          self.builder().emit(LoadNone, span);
        }
        self.builder().emit(Return, span);
      }
      ast::Ctrl::Yield(stmt) => {
        if let Some(value) = stmt.value.as_ref() {
          self.emit_expr(value);
        } else {
          self.builder().emit(LoadNone, span);
        }
        self.builder().emit(Yield, span);
      }
      ast::Ctrl::Continue => {
        let function = self.current_function();
        let loop_ = function
          .current_loop
          .as_ref()
          .expect("attempted to emit continue outside of loop");
        function.builder.emit_jump_loop(&loop_.start, span);
      }
      ast::Ctrl::Break => {
        let function = self.current_function();
        let loop_ = function
          .current_loop
          .as_ref()
          .expect("attempted to emit continue outside of loop");
        function.builder.emit_jump(&loop_.end, span);
      }
    }
  }

  fn emit_func_stmt(&mut self, stmt: &'src ast::Func<'src>) {
    let function = self.emit_function(stmt);
    let desc = self.constant_value(function);
    self.builder().emit(MakeFn { desc }, stmt.name.span);
    self.emit_var(stmt.name.lexeme(), stmt.name.span);
  }

  fn emit_class_initializer(
    class: &'src ast::Class<'src>,
  ) -> impl FnOnce(&mut State<'cx, 'src>, Register) {
    |this, receiver| this.emit_class_fields(receiver, &class.members.fields, class.name.span)
  }

  // TODO: call parent initializer
  // child class initializer should inherit parent class initializer params

  fn emit_class_fake_initializer(
    &mut self,
    class: &'src ast::Class<'src>,
  ) -> Ptr<object::FunctionDescriptor> {
    let span = class.name.span;

    self.module.functions.push(Function::new(
      self.cx,
      "init",
      function::Params {
        has_self: true,
        min: 0,
        max: 0,
      },
      false,
    ));

    let receiver = self.alloc_register();
    self.emit_class_fields(receiver, &class.members.fields, span);

    self.builder().emit(LoadNone, span);
    self.builder().emit(Return, span);

    let function = self.module.functions.pop().unwrap().finish();

    self
      .current_function()
      .inner_functions
      .push(function.clone());

    function
  }

  fn emit_class_fields(
    &mut self,
    receiver: Register,
    fields: &'src [ast::Field<'src>],
    span: Span,
  ) {
    for field in fields.iter() {
      let name = self.constant_name(&field.name);
      self.emit_expr(&field.default);
      let obj = receiver.access();
      self.builder().emit(StoreField { obj, name }, span);
    }
  }

  fn emit_class_stmt(&mut self, stmt: &'src ast::Class<'src>) {
    let mut init = None;
    let mut meta_methods = IndexMap::with_capacity(stmt.members.meta.len());
    for (key, func) in stmt.members.meta.iter() {
      if *key == ast::Meta::Init {
        let func = self.emit_function_with_prelude(func, Self::emit_class_initializer(stmt));
        init = Some(func);
      } else {
        let func = self.emit_function(func);
        meta_methods.insert(*key, func);
      }
    }

    let init = match init {
      Some(init) => init,
      None => self.emit_class_fake_initializer(stmt),
    };

    let mut methods = IndexMap::with_capacity(stmt.members.methods.len());
    for func in stmt.members.methods.iter() {
      let func = self.emit_function(func);
      methods.insert(func.name.clone(), func);
    }

    let class = self.cx.alloc(object::ClassDescriptor {
      name: self.cx.intern(stmt.name.to_string()),
      params: function::Params::from_ast_class(stmt),
      is_derived: stmt.parent.is_some(),
      init,
      meta_methods,
      methods,
    });
    let desc = self.constant_value(class);

    if let Some(parent) = &stmt.parent {
      self.emit_get(parent.lexeme(), parent.span);
      self
        .builder()
        .emit(MakeClassDerived { desc }, stmt.name.span);
    } else {
      self.builder().emit(MakeClass { desc }, stmt.name.span);
    }

    self.emit_var(stmt.name.lexeme(), stmt.name.span);
  }

  fn emit_expr_stmt(&mut self, expr: &'src ast::Expr<'src>) {
    self.emit_expr(expr)
  }

  fn emit_pass_stmt(&mut self) {}

  fn emit_print_stmt(&mut self, stmt: &'src ast::Print<'src>, span: Span) {
    match &stmt.values[..] {
      [] => {}
      [value] => {
        self.emit_expr(value);
        self.builder().emit(Print, span);
      }
      values => {
        let args = self.alloc_register_slice(values.len());

        for (i, value) in values.iter().enumerate() {
          self.emit_expr(value);
          self.builder().emit(
            Store {
              reg: args.access(i),
            },
            span,
          );
        }

        self.builder().emit(
          PrintN {
            start: args.access(0),
            count: op::Count(args.len() as u32),
          },
          span,
        );
      }
    }
  }

  fn emit_import_stmt(&mut self, stmt: &'src ast::Import<'src>, span: Span) {
    match stmt {
      ast::Import::Module { path, alias } => {
        let name = alias.as_ref().unwrap_or(path.last().unwrap());
        let path = path.iter().map(|p| p.as_ref()).join(".");
        let path = self.constant_name(path);
        let dst = self.alloc_register();
        self.declare_local(name.lexeme(), dst.clone());
        self.builder().emit(
          Import {
            path,
            dst: dst.access(),
          },
          span,
        );
      }
      ast::Import::Symbols { path, symbols } => {
        let path = path.iter().map(|p| p.as_ref()).join(".");
        let path = self.constant_name(path);
        let temp = self.alloc_register();
        self.builder().emit(
          Import {
            path,
            dst: temp.access(),
          },
          span,
        );

        for symbol in symbols {
          let name = symbol.alias.as_ref().unwrap_or(&symbol.name);
          let name_idx = self.constant_name(name);

          self.builder().emit(Load { reg: temp.access() }, span);
          self.builder().emit(LoadField { name: name_idx }, span);

          let dst = self.alloc_register();
          self.declare_local(name.lexeme(), dst.clone());
          self.builder().emit(Store { reg: dst.access() }, span);
        }
      }
    }
  }
}
