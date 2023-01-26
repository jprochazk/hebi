use std::ops::Deref;

use beef::lean::Cow;
use indexmap::IndexMap;
use op::instruction::*;
use syntax::ast;
use value::object::{func, Func};
use value::Value;

pub fn emit<'src>(
  ctx: &Context,
  name: impl Into<Cow<'src, str>>,
  module: &'src ast::Module<'src>,
) -> Result<func::Func> {
  Emitter::new(ctx, name, module)
    .emit_main()
    .map(|result| result.func)
}

use crate::regalloc::{RegAlloc, Register};
use crate::{Context, Error, Result};

struct Emitter<'src> {
  state: Function<'src>,
  module: &'src ast::Module<'src>,
  ctx: Context,
}

struct EmitResult<'src> {
  func: Func,
  captures: IndexMap<Cow<'src, str>, Upvalue>,
}

impl<'src> Emitter<'src> {
  fn new(ctx: &Context, name: impl Into<Cow<'src, str>>, module: &'src ast::Module<'src>) -> Self {
    Self {
      state: Function::new(name, None),
      module,
      ctx: ctx.clone(),
    }
  }

  fn emit_main(mut self) -> Result<EmitResult<'src>> {
    for stmt in self.module.body.iter() {
      self.emit_stmt(stmt)?;
    }
    self.emit_op(Ret);

    let (frame_size, register_map) = self.state.regalloc.scan();

    self.state.builder.patch(|instructions| {
      for instruction in instructions.iter_mut() {
        op::instruction::update_registers(instruction, &register_map)
      }
    });

    let Chunk {
      name,
      bytecode,
      const_pool,
    } = self.state.builder.build();

    Ok(EmitResult {
      func: Func::new(
        name,
        frame_size,
        bytecode,
        const_pool,
        func::Params {
          min: 0,
          max: None,
          kw: Default::default(),
        },
      ),
      captures: Default::default(),
    })
  }

  fn emit_func(
    &mut self,
    name: impl Into<Cow<'src, str>>,
    params: func::Params,
    f: impl FnOnce(&mut Self) -> Result<()>,
  ) -> Result<EmitResult<'src>> {
    let next = Function::new(name.into(), None);
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

    let Function {
      mut builder,
      regalloc,
      captures,
      ..
    } = next;

    let (frame_size, register_map) = regalloc.scan();

    builder.patch(|instructions| {
      for instruction in instructions.iter_mut() {
        op::instruction::update_registers(instruction, &register_map)
      }
    });

    let Chunk {
      name,
      bytecode,
      const_pool,
    } = builder.build();

    Ok(EmitResult {
      func: Func::new(name, frame_size, bytecode, const_pool, params),
      captures,
    })
  }

  fn const_name(&mut self, str: &str) -> u32 {
    self.state.builder.constant(self.ctx.alloc_string(str))
  }

  fn const_value(&mut self, value: impl Into<Value>) -> u32 {
    let value: Value = value.into();
    if value.is_string() {
      panic!("use `const_name` instead of `const_value` for constant strings");
    }
    self.state.builder.constant(value)
  }

  fn emit_op(&mut self, op: impl Into<Instruction>) {
    self.state.builder.op(op)
  }

  fn reg(&mut self) -> Register {
    self.state.regalloc.alloc()
  }

  fn label(&mut self, name: impl Into<Cow<'static, str>>) -> LabelId {
    self.state.builder.label(name)
  }

  fn labels<const N: usize, T: Into<Cow<'static, str>> + Clone>(
    &mut self,
    names: [T; N],
  ) -> [LabelId; N] {
    self.state.builder.labels(names)
  }

  fn finish_label(&mut self, label: LabelId) {
    self.state.builder.finish_label(label)
  }

  fn resolve_var(&mut self, name: impl Into<Cow<'src, str>>) -> Get {
    let name = name.into();

    if let Some(reg) = self.state.local(&name) {
      return Get::Local(reg);
    }

    if let Some(reg) = self.state.capture(name) {
      return Get::Capture(reg);
    }

    Get::Global
  }
}

enum Get {
  Local(Register),
  Capture(u32),
  Global,
}

struct Upvalue {
  /// Capture slot.
  ///
  /// Each closure object contains a `captures` list, which will be indexed by
  /// this value.
  slot: u32,
  /// Describes how to capture the value in the enclosing scope
  kind: UpvalueKind,
}

enum UpvalueKind {
  Register(Register),
  Capture(u32),
}

struct Function<'src> {
  builder: Builder<Value>,
  name: Cow<'src, str>,
  parent: Option<Box<Function<'src>>>,
  regalloc: RegAlloc,

  // TODO: use a better data structure for this
  /// Map of variable name to register.
  ///
  /// Because locals may shadow other locals, the register is actually a stack
  /// of registers, and only the last one is active.
  ///
  /// Invariants:
  /// - Register stacks may not be empty. If a stack is about to be emptied, the
  ///   local should be removed instead.
  /// - The top-level `Vec` may not be empty. There must always be at least one
  ///   scope.
  locals: Vec<IndexMap<Cow<'src, str>, Vec<Register>>>,
  /// List of variables captured from an enclosing scope.
  ///
  /// These may be shadowed by a local.
  ///
  /// Invariants:
  /// - Captures may never be removed.
  /// - The stored registers may not be used until the current function has been
  ///   emitted.
  captures: IndexMap<Cow<'src, str>, Upvalue>,
  capture_slot: u32,

  /// Whether or not we're currently emitting `?<expr>`.
  ///
  /// This changes `LoadNamed`/`LoadKeyed` to `LoadNamedOpt`/`LoadKeyedOpt`,
  /// which return `none` instead of panicking if a field doesn't exist.
  is_in_opt_expr: bool,
}

impl<'src> Function<'src> {
  fn new(name: impl Into<Cow<'src, str>>, parent: Option<Box<Function<'src>>>) -> Self {
    let name = name.into();
    Self {
      builder: Builder::new(name.to_string()),
      name,
      parent,
      regalloc: RegAlloc::new(),
      locals: vec![IndexMap::new()],
      captures: IndexMap::new(),
      capture_slot: 0,
      is_in_opt_expr: false,
    }
  }

  fn begin_scope(&mut self) {
    self.locals.push(IndexMap::new());
  }

  fn is_global_scope(&self) -> bool {
    self.parent.is_none() && self.locals.len() < 2
  }

  fn end_scope(&mut self) {
    self
      .locals
      .pop()
      .expect("end_scope should not empty the locals stack");
  }

  fn declare_local(&mut self, name: impl Into<Cow<'src, str>>, reg: Register) {
    reg.index(); // ensure liveness at time of declaration
    let name = name.into();
    if let Some(stack) = self
      .locals
      .last_mut()
      .expect("scope stack may not be empty")
      .get_mut(&name)
    {
      stack.push(reg);
    } else {
      self
        .locals
        .last_mut()
        .expect("scope stack may not be empty")
        .insert(name, vec![reg]);
    }
  }

  fn local(&self, name: &str) -> Option<Register> {
    let stack = self.locals.iter().rev().find_map(|stack| stack.get(name))?;
    let reg = stack.last().unwrap_or_else(|| {
      panic!(
        "local's register stack {name} in function {} is empty",
        self.name
      )
    });
    Some(reg.clone())
  }

  fn capture(&mut self, name: impl Into<Cow<'src, str>>) -> Option<u32> {
    let name = name.into();
    let Some(parent) = self.parent.as_deref_mut() else {
      return None;
    };

    if let Some(info) = self.captures.get(&name) {
      return Some(info.slot);
    }

    let local_slot = self.capture_slot;
    self.capture_slot += 1;
    if let Some(reg) = parent.local(&name) {
      self.captures.insert(
        name,
        Upvalue {
          slot: local_slot,
          kind: UpvalueKind::Register(reg),
        },
      );
      return Some(local_slot);
    }

    if let Some(parent_slot) = parent.capture(name.clone()) {
      self.captures.insert(
        name,
        Upvalue {
          slot: local_slot,
          kind: UpvalueKind::Capture(parent_slot),
        },
      );
      return Some(local_slot);
    }

    None
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
        ast::StmtKind::Import(v) => self.emit_import_stmt(v),
      }
    }

    fn emit_var_stmt(&mut self, stmt: &'src ast::Var<'src>) -> Result<()> {
      self.emit_expr(&stmt.value)?;
      if self.state.is_global_scope() {
        let name = self.const_name(&stmt.name);
        self.emit_op(StoreGlobal { name });
      } else {
        let reg = self.reg();
        self.emit_op(StoreReg { reg: reg.index() });
        self.state.declare_local(stmt.name.deref().clone(), reg);
      }
      Ok(())
    }

    fn emit_if_stmt(&mut self, stmt: &'src ast::If<'src>) -> Result<()> {
      // exit label for all branches
      let end = self.label("end");

      for branch in stmt.branches.iter() {
        let next = self.label("next");
        self.emit_expr(&branch.cond)?;
        self.emit_op(JumpIfFalse { offset: next.id() });
        self.state.begin_scope();
        for stmt in branch.body.iter() {
          self.emit_stmt(stmt)?;
        }
        self.emit_op(Jump { offset: end.id() });
        self.state.end_scope();
        self.finish_label(next);
      }

      if let Some(default) = stmt.default.as_ref() {
        self.state.begin_scope();
        for stmt in default.iter() {
          self.emit_stmt(stmt)?;
        }
        self.state.end_scope();
      }

      self.finish_label(end);

      Ok(())
    }

    fn emit_loop_stmt(&mut self, stmt: &'src ast::Loop<'src>) -> Result<()> {
      todo!()
    }

    fn emit_ctrl_stmt(&mut self, stmt: &'src ast::Ctrl<'src>) -> Result<()> {
      match stmt {
        ast::Ctrl::Return(v) => {
          if let Some(value) = v.value.as_ref() {
            self.emit_expr(value)?;
          } else {
            self.emit_op(PushNone);
          }
          self.emit_op(Ret);
        }
        ast::Ctrl::Yield(_) => todo!(),
        ast::Ctrl::Continue => todo!(),
        ast::Ctrl::Break => todo!(),
      }

      Ok(())
    }

    fn emit_func_stmt(&mut self, stmt: &'src ast::Func<'src>) -> Result<()> {
      let name = stmt.name.deref().clone();
      let params = func::Params {
        // min = number of positional arguments without defaults
        min: stmt.params.pos.iter().filter(|v| v.1.is_none()).count() as u32,
        // max = number of positional arguments OR none, if `argv` exists
        max: if stmt.params.argv.is_some() {
          None
        } else {
          Some(stmt.params.pos.len() as u32)
        },
        kw: stmt
          .params
          .kw
          .iter()
          .filter(|(_, default)| default.is_none())
          .map(|(key, _)| String::from(key.as_ref()))
          .collect(),
      };

      let result = self.emit_func(name.clone(), params, |this| {
        this.emit_func_params(stmt)?;

        for stmt in stmt.body.iter() {
          this.emit_stmt(stmt)?;
        }

        Ok(())
      })?;

      if result.captures.is_empty() {
        let func = self.const_value(result.func);
        self.emit_op(LoadConst { slot: func });
      } else {
        let descriptor = self.const_value(func::ClosureDescriptor {
          func: result.func,
          num_captures: result.captures.len() as u32,
        });
        self.emit_op(CreateClosure { descriptor });
        for (_, info) in result.captures.iter() {
          match &info.kind {
            UpvalueKind::Register(reg) => self.emit_op(CaptureReg { reg: reg.index() }),
            UpvalueKind::Capture(slot) => self.emit_op(CaptureSlot { slot: *slot }),
          };
        }
      }

      if self.state.is_global_scope() {
        let name = self.const_name(&name);
        self.emit_op(StoreGlobal { name });
      } else {
        let reg = self.reg();
        self.emit_op(StoreReg { reg: reg.index() });
        self.state.declare_local(name, reg);
      }

      Ok(())
    }

    fn emit_func_params(&mut self, func: &'src ast::Func<'src>) -> Result<()> {
      // NOTE: params are already checked by `call` instruction

      // allocate registers
      let receiver = self.reg();
      let this_func = self.reg();
      let argv = self.reg();
      let kwargs = self.reg();
      let pos = func
        .params
        .pos
        .iter()
        .map(|p| (p, self.reg()))
        .collect::<Vec<_>>();
      let kw = func
        .params
        .kw
        .iter()
        .map(|p| (p, self.reg()))
        .collect::<Vec<_>>();

      // only pos params with defaults need emit
      // invariants:
      // - the `call` instruction checks that all required params are present
      // - required params may not appear after optional params
      // - keyword args
      // emit pos defaults
      for (i, ((_, default), reg)) in pos.iter().enumerate() {
        if let Some(default) = default {
          let next = self.label("next");
          self.emit_op(IsPosParamNotSet { index: i as u32 });
          self.emit_op(JumpIfFalse { offset: next.id() });
          self.emit_expr(default)?;
          self.emit_op(StoreReg { reg: reg.index() });
          self.finish_label(next);
        }
      }
      // emit kw + defaults
      for ((name, default), reg) in kw.iter() {
        let name = self.const_name(name);
        // #if param.default.is_some()
        if let Some(default) = default {
          // if not #(param.name) in kw:
          let from_key = self.label("next");
          self.emit_op(IsKwParamNotSet { name });
          self.emit_op(JumpIfFalse {
            offset: from_key.id(),
          });
          // store(param.reg) = #(param.default)
          self.emit_expr(default)?;
          self.emit_op(StoreReg { reg: reg.index() });
          self.finish_label(from_key);
          // else:
          // store(param.reg) = kw.remove(#(param.name))
          self.emit_op(LoadKwParam {
            name,
            param: reg.index(),
          });
        } else {
          // store(param.reg) = kw.remove(#(param.name))
          self.emit_op(LoadKwParam {
            name,
            param: reg.index(),
          });
        }
      }

      // declare locals
      for ((name, _), reg) in kw.iter().rev() {
        self.state.declare_local(name.deref().clone(), reg.clone())
      }
      for ((name, _), reg) in pos.iter().rev() {
        self.state.declare_local(name.deref().clone(), reg.clone())
      }
      if let Some(name) = func.params.kwargs.as_ref() {
        self.state.declare_local(name.deref().clone(), kwargs);
      }
      if let Some(name) = func.params.argv.as_ref() {
        self.state.declare_local(name.deref().clone(), argv);
      }
      self
        .state
        .declare_local(func.name.deref().clone(), this_func);
      self.state.declare_local("self", receiver);

      Ok(())
    }

    fn emit_class_stmt(&mut self, stmt: &'src ast::Class<'src>) -> Result<()> {
      todo!()
    }

    fn emit_expr_stmt(&mut self, expr: &'src ast::Expr<'src>) -> Result<()> {
      self.emit_expr(expr)
    }

    fn emit_pass_stmt(&mut self) -> Result<()> {
      Ok(())
    }

    fn emit_print_stmt(&mut self, stmt: &'src ast::Print<'src>) -> Result<()> {
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

      if stmt.values.len() == 1 {
        self.emit_expr(&stmt.values[0])?;
        self.emit_op(Print);
      } else {
        let temp = self.reg();
        self.emit_op(CreateEmptyList);
        self.emit_op(StoreReg { reg: temp.index() });
        for value in stmt.values.iter() {
          self.emit_expr(value)?;
          self.emit_op(PushToList { list: temp.index() });
        }
        self.emit_op(PrintList { list: temp.index() });
      }

      Ok(())
    }

    fn emit_import_stmt(&mut self, stmt: &'src ast::Import<'src>) -> Result<()> {
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
    pub(crate) fn emit_expr(&mut self, expr: &'src ast::Expr<'src>) -> Result<()> {
      match expr.deref() {
        ast::ExprKind::Literal(v) => self.emit_literal_expr(v),
        ast::ExprKind::Binary(v) => self.emit_binary_expr(v),
        ast::ExprKind::Unary(v) => self.emit_unary_expr(v),
        ast::ExprKind::GetVar(v) => self.emit_get_var_expr(v),
        ast::ExprKind::SetVar(v) => self.emit_set_var_expr(v),
        ast::ExprKind::GetNamed(v) => self.emit_get_named_expr(v),
        ast::ExprKind::SetNamed(v) => self.emit_set_named_expr(v),
        ast::ExprKind::GetKeyed(v) => self.emit_get_keyed_expr(v),
        ast::ExprKind::SetKeyed(v) => self.emit_set_keyed_expr(v),
        ast::ExprKind::Yield(v) => self.emit_yield_expr(v),
        ast::ExprKind::Call(v) => self.emit_call_expr(v),
      }
    }

    fn emit_literal_expr(&mut self, expr: &'src ast::Literal<'src>) -> Result<()> {
      match expr {
        ast::Literal::None => self.emit_op(PushNone),
        ast::Literal::Int(v) => self.emit_op(PushSmallInt { value: *v }),
        ast::Literal::Float(v) => {
          // float is 4 bits so cannot be stored inline,
          // but it is interned
          let num = self.const_value(*v);
          self.emit_op(LoadConst { slot: num });
        }
        ast::Literal::Bool(v) => match v {
          true => self.emit_op(PushTrue),
          false => self.emit_op(PushFalse),
        },
        ast::Literal::String(v) => {
          // `const_` interns the string
          let str = self.const_name(v);
          self.emit_op(LoadConst { slot: str });
        }
        ast::Literal::List(list) => {
          // TODO: from descriptor
          let temp = self.reg();
          self.emit_op(CreateEmptyList);
          self.emit_op(StoreReg { reg: temp.index() });
          for v in list {
            self.emit_expr(v)?;
            self.emit_op(PushToList { list: temp.index() });
          }
          self.emit_op(LoadReg { reg: temp.index() });
        }
        ast::Literal::Dict(obj) => {
          // TODO: from descriptor
          let temp = self.reg();
          self.emit_op(CreateEmptyDict);
          self.emit_op(StoreReg { reg: temp.index() });
          for (k, v) in obj {
            let key_reg = self.reg();
            self.emit_expr(k)?;
            self.emit_op(StoreReg {
              reg: key_reg.index(),
            });
            self.emit_expr(v)?;
            // TODO: use `InsertToDictKeyed for constant keys`
            self.emit_op(InsertToDict {
              key: key_reg.index(),
              dict: temp.index(),
            });
          }
          self.emit_op(LoadReg { reg: temp.index() });
        }
      }
      Ok(())
    }

    fn emit_binary_expr(&mut self, expr: &'src ast::Binary<'src>) -> Result<()> {
      // binary expressions store lhs in a register,
      // and rhs in the accumulator

      match expr.op {
        ast::BinaryOp::And | ast::BinaryOp::Or | ast::BinaryOp::Maybe => {
          return self.emit_logical_expr(expr)
        }
        _ => {}
      }

      let lhs = self.reg();
      self.emit_expr(&expr.left)?;
      self.emit_op(StoreReg { reg: lhs.index() });
      self.emit_expr(&expr.right)?;

      let lhs = lhs.index();
      match expr.op {
        ast::BinaryOp::Add => self.emit_op(Add { lhs }),
        ast::BinaryOp::Sub => self.emit_op(Sub { lhs }),
        ast::BinaryOp::Div => self.emit_op(Div { lhs }),
        ast::BinaryOp::Mul => self.emit_op(Mul { lhs }),
        ast::BinaryOp::Rem => self.emit_op(Rem { lhs }),
        ast::BinaryOp::Pow => self.emit_op(Pow { lhs }),
        ast::BinaryOp::Eq => self.emit_op(CmpEq { lhs }),
        ast::BinaryOp::Neq => self.emit_op(CmpNeq { lhs }),
        ast::BinaryOp::More => self.emit_op(CmpGt { lhs }),
        ast::BinaryOp::MoreEq => self.emit_op(CmpGe { lhs }),
        ast::BinaryOp::Less => self.emit_op(CmpLt { lhs }),
        ast::BinaryOp::LessEq => self.emit_op(CmpLe { lhs }),
        ast::BinaryOp::And | ast::BinaryOp::Or | ast::BinaryOp::Maybe => unreachable!(),
      }

      Ok(())
    }

    fn emit_logical_expr(&mut self, expr: &'src ast::Binary<'src>) -> Result<()> {
      match expr.op {
        ast::BinaryOp::And => {
          /*
            <left> && <right>
            v = <left>
            if v:
              v = <right>
          */
          let end = self.label("end");
          self.emit_expr(&expr.left)?;
          self.emit_op(JumpIfFalse { offset: end.id() });
          self.emit_expr(&expr.right)?;
          self.finish_label(end);
        }
        ast::BinaryOp::Or => {
          /*
            <left> || <right>
            v = <left>
            if !v:
              v = <right>
          */
          let [rhs, end] = self.labels(["rhs", "end"]);
          self.emit_expr(&expr.left)?;
          self.emit_op(JumpIfFalse { offset: rhs.id() });
          self.emit_op(Jump { offset: end.id() });
          self.finish_label(rhs);
          self.emit_expr(&expr.right)?;
          self.finish_label(end);
        }
        ast::BinaryOp::Maybe => {
          /*
            <left> ?? <right>
            v = <left>
            if v is none:
              v = <right>
          */
          let end = self.label("end");
          self.emit_expr(&expr.left)?;
          self.emit_op(IsNone);
          self.emit_op(JumpIfFalse { offset: end.id() });
          self.emit_expr(&expr.right)?;
          self.finish_label(end);
        }
        _ => unreachable!("not a logical expr: {:?}", expr.op),
      }

      Ok(())
    }

    fn emit_unary_expr(&mut self, expr: &'src ast::Unary<'src>) -> Result<()> {
      // unary expressions only use the accumulator

      if matches!(expr.op, ast::UnaryOp::Opt) {
        return self.emit_opt_expr(expr);
      }

      self.emit_expr(&expr.right)?;

      match expr.op {
        ast::UnaryOp::Plus => self.emit_op(UnaryPlus),
        ast::UnaryOp::Minus => self.emit_op(UnaryMinus),
        ast::UnaryOp::Not => self.emit_op(UnaryNot),
        ast::UnaryOp::Opt => unreachable!(),
      }

      Ok(())
    }

    fn emit_opt_expr(&mut self, expr: &'src ast::Unary<'src>) -> Result<()> {
      assert!(matches!(expr.op, ast::UnaryOp::Opt));

      // - emit_call_expr <- with receiver, `CallMethodOpt` or similar

      let prev = std::mem::replace(&mut self.state.is_in_opt_expr, true);
      self.emit_expr(&expr.right)?;
      let _ = std::mem::replace(&mut self.state.is_in_opt_expr, prev);

      Ok(())
    }

    fn emit_get_var_expr(&mut self, expr: &'src ast::GetVar<'src>) -> Result<()> {
      match self.resolve_var(expr.name.deref().clone()) {
        Get::Local(reg) => self.emit_op(LoadReg { reg: reg.index() }),
        Get::Capture(slot) => self.emit_op(LoadCapture { slot }),
        Get::Global => {
          let name = self.const_name(&expr.name);
          self.emit_op(LoadGlobal { name })
        }
      }

      Ok(())
    }

    fn emit_set_var_expr(&mut self, expr: &'src ast::SetVar<'src>) -> Result<()> {
      self.emit_expr(&expr.value)?;
      match self.resolve_var(expr.target.name.deref().clone()) {
        Get::Local(reg) => self.emit_op(StoreReg { reg: reg.index() }),
        Get::Capture(slot) => self.emit_op(StoreCapture { slot }),
        Get::Global => {
          let name = self.const_name(&expr.target.name);
          self.emit_op(StoreGlobal { name });
        }
      }

      Ok(())
    }

    fn emit_get_named_expr(&mut self, expr: &'src ast::GetNamed<'src>) -> Result<()> {
      let name = self.const_name(&expr.name);
      self.emit_expr(&expr.target)?;
      if self.state.is_in_opt_expr {
        self.emit_op(LoadNamedOpt { name });
      } else {
        self.emit_op(LoadNamed { name });
      }

      Ok(())
    }

    fn emit_set_named_expr(&mut self, expr: &'src ast::SetNamed<'src>) -> Result<()> {
      let obj = self.reg();
      let name = self.const_name(&expr.target.name);
      self.emit_expr(&expr.target.target)?;
      self.emit_op(StoreReg { reg: obj.index() });
      self.emit_expr(&expr.value)?;
      self.emit_op(StoreNamed {
        name,
        obj: obj.index(),
      });

      Ok(())
    }

    fn emit_get_keyed_expr(&mut self, expr: &'src ast::GetKeyed<'src>) -> Result<()> {
      let key = self.reg();
      self.emit_expr(&expr.key)?;
      self.emit_op(StoreReg { reg: key.index() });
      self.emit_expr(&expr.target)?;
      if self.state.is_in_opt_expr {
        self.emit_op(LoadKeyedOpt { key: key.index() });
      } else {
        self.emit_op(LoadKeyed { key: key.index() });
      }

      Ok(())
    }

    fn emit_set_keyed_expr(&mut self, expr: &'src ast::SetKeyed<'src>) -> Result<()> {
      let obj = self.reg();
      let key = self.reg();
      self.emit_expr(&expr.target.key)?;
      self.emit_op(StoreReg { reg: key.index() });
      self.emit_expr(&expr.target.target)?;
      self.emit_op(StoreReg { reg: obj.index() });
      self.emit_expr(&expr.value)?;
      self.emit_op(StoreKeyed {
        key: key.index(),
        obj: obj.index(),
      });

      Ok(())
    }

    fn emit_yield_expr(&mut self, expr: &'src ast::Yield<'src>) -> Result<()> {
      todo!()
    }

    fn emit_call_expr(&mut self, expr: &'src ast::Call<'src>) -> Result<()> {
      let callee = self.reg();
      self.emit_expr(&expr.target)?;
      self.emit_op(StoreReg {
        reg: callee.index(),
      });

      let mut kw = None;
      if !expr.args.kw.is_empty() {
        let kw_reg = self.reg();
        self.emit_op(CreateEmptyDict);
        self.emit_op(StoreReg {
          reg: kw_reg.index(),
        });
        kw = Some(kw_reg);
      }

      // allocate registers for args, then emit them
      // this ensures that the args are contiguous on the stack
      let argv = (0..expr.args.pos.len())
        .map(|_| self.reg())
        .collect::<Vec<_>>();
      for (reg, value) in argv.iter().zip(expr.args.pos.iter()) {
        self.emit_expr(value)?;
        self.emit_op(StoreReg { reg: reg.index() });
      }

      for (key, value) in expr.args.kw.iter() {
        let key = self.const_name(key);
        self.emit_expr(value)?;
        self.emit_op(InsertToDictKeyed {
          key,
          dict: kw.as_ref().unwrap().index(),
        });
      }

      // ensure liveness of:
      // - args (in reverse)
      // - kw dict
      // - callee
      for a in argv.iter().rev() {
        a.index();
      }
      if let Some(kw) = &kw {
        kw.index();
      }
      callee.index();

      if kw.is_none() {
        self.emit_op(Call {
          callee: callee.index(),
          args: argv.len() as u32,
        });
      } else {
        self.emit_op(CallKw {
          callee: callee.index(),
          args: argv.len() as u32,
        });
      }

      Ok(())
    }
  }
}

#[cfg(test)]
mod tests;
