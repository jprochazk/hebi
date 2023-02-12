use std::ops::Deref;

use beef::lean::Cow;
use indexmap::IndexMap;
use op::*;
use syntax::ast;
use value::object::handle::Handle;
use value::object::{func, Func, Module};
use value::Value;

pub fn emit<'src>(
  ctx: &Context,
  name: impl Into<Cow<'src, str>>,
  module: &'src ast::Module<'src>,
) -> Result<Handle<Module>> {
  let name = name.into();
  let result = Emitter::new(ctx, name.clone(), module).emit_main()?;

  Ok(Module::new(name.to_string(), result.func.into()).into())
}

// TODO: make infallible
// TODO: do not emit argv/kwargs registers if they are unused

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
  #[cfg(test)]
  regalloc: RegAlloc,
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
    // TODO: remove these registers (they aren't needed)
    let regs = [self.reg(), self.reg(), self.reg(), self.reg()]
      .into_iter()
      .map(Some);
    self.state.preserve.extend(regs);

    for stmt in self.module.body.iter() {
      self.emit_stmt(stmt)?;
    }
    self.emit_op(Ret);

    for reg in self.state.preserve.iter().rev().filter_map(|v| v.as_ref()) {
      reg.index();
    }

    let (frame_size, register_map) = self.state.regalloc.scan();
    //panic!("{register_map:?}");

    self.state.builder.patch(|instructions| {
      for instruction in instructions.iter_mut() {
        op::update_registers(instruction, &register_map)
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
          has_self: false,
          min: 0,
          max: 0,
          argv: false,
          kwargs: false,
          pos: Default::default(),
          kw: Default::default(),
        },
      ),
      captures: Default::default(),
      #[cfg(test)]
      regalloc: self.state.regalloc.clone(),
    })
  }

  fn emit_func(&mut self, func: &'src ast::Func<'src>) -> Result<EmitResult<'src>> {
    let params = ast_params_to_func_params(&func.params);
    let next = Function::new(func.name.deref().clone(), None);
    let parent = std::mem::replace(&mut self.state, next);
    self.state.parent = Some(Box::new(parent));

    fn emit_func_inner<'src>(this: &mut Emitter<'src>, stmt: &'src ast::Func<'src>) -> Result<()> {
      this.emit_func_params(stmt)?;

      for stmt in stmt.body.iter() {
        this.emit_stmt(stmt)?;
      }

      for reg in this.state.preserve.iter().rev().filter_map(|v| v.as_ref()) {
        reg.index();
      }

      if !matches!(
        this.state.builder.instructions().last(),
        Some(Instruction::Ret(..))
      ) {
        this.emit_op(PushNone);
        this.emit_op(Ret);
      }

      Ok(())
    }

    let result = emit_func_inner(self, func);

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
        op::update_registers(instruction, &register_map)
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
      #[cfg(test)]
      regalloc: self.state.regalloc.clone(),
    })
  }

  fn emit_func_params(&mut self, func: &'src ast::Func<'src>) -> Result<()> {
    // NOTE: params are already checked by `call` instruction

    // allocate registers
    let this_func = self.reg();
    let argv = self.reg();
    let kwargs = self.reg();
    let receiver = self.reg();
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

    // preserve the first 4
    self.state.preserve.extend([
      Some(this_func.clone()),
      Some(argv.clone()),
      Some(kwargs.clone()),
      Some(receiver.clone()),
    ]);

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
        let [from_key, next] = self.labels(["from_key", "next"]);
        self.emit_op(IsKwParamNotSet { name });
        self.emit_op(JumpIfFalse {
          offset: from_key.id(),
        });
        // store(param.reg) = #(param.default)
        self.emit_expr(default)?;
        self.emit_op(StoreReg { reg: reg.index() });
        self.emit_op(Jump { offset: next.id() });
        self.finish_label(from_key);
        // else:
        // store(param.reg) = kw.remove(#(param.name))
        self.emit_op(LoadKwParam {
          name,
          param: reg.index(),
        });
        self.finish_label(next);
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
    if func.params.has_self {
      self.state.declare_local("self", receiver);
    }

    Ok(())
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

fn ast_params_to_func_params(params: &ast::Params) -> func::Params {
  func::Params {
    has_self: params.has_self,
    // min = number of positional arguments without defaults
    min: params.pos.iter().filter(|v| v.1.is_none()).count(),
    // max = number of positional arguments OR none, if `argv` exists
    max: params.pos.len(),
    argv: params.argv.is_some(),
    kwargs: params.kwargs.is_some(),
    pos: params.pos.iter().map(|v| v.0.deref().to_string()).collect(),
    kw: params
      .kw
      .iter()
      .map(|(key, v)| (String::from(key.as_ref()), v.is_some()))
      .collect(),
  }
}

fn ast_class_to_params(class: &ast::Class) -> func::Params {
  class
    .methods
    .iter()
    .find(|m| m.name.deref() == "init")
    .map(|m| ast_params_to_func_params(&m.params))
    .unwrap_or_else(|| func::Params {
      has_self: false,
      min: 0,
      max: 0,
      argv: false,
      kwargs: false,
      pos: vec![],
      kw: class
        .fields
        .iter()
        .map(|f| (f.name.to_string(), f.default.is_some()))
        .collect(),
    })
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

struct Loop {
  start: LabelId,
  end: LabelId,
}

struct Function<'src> {
  builder: Builder<Value>,
  name: Cow<'src, str>,
  parent: Option<Box<Function<'src>>>,
  regalloc: RegAlloc,

  /// List of registers to keep alive for the entire scope of the function.
  preserve: Vec<Option<Register>>,

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

  loops: Vec<Loop>,
}

impl<'src> Function<'src> {
  fn new(name: impl Into<Cow<'src, str>>, parent: Option<Box<Function<'src>>>) -> Self {
    let name = name.into();
    Self {
      builder: Builder::new(name.to_string()),
      name,
      parent,
      regalloc: RegAlloc::new(),
      preserve: vec![],
      locals: vec![IndexMap::new()],
      captures: IndexMap::new(),
      capture_slot: 0,
      is_in_opt_expr: false,
      loops: vec![],
    }
  }

  fn begin_loop(&mut self, start: LabelId, end: LabelId) {
    self.loops.push(Loop { start, end })
  }

  fn current_loop(&self) -> Option<&Loop> {
    self.loops.last()
  }

  fn end_loop(&mut self) {
    self.loops.pop();
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
  use value::object::{class, module};

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
      match stmt {
        ast::Loop::For(v) => match &v.iter {
          ast::ForIter::Range(range) => self.emit_for_range_loop(v, range),
          ast::ForIter::Expr(iter) => self.emit_for_iter_loop(v, iter),
        },
        ast::Loop::While(v) => self.emit_while_loop(v),
        ast::Loop::Infinite(v) => self.emit_inf_loop(v),
      }
    }

    fn emit_for_range_loop(
      &mut self,
      stmt: &'src ast::For<'src>,
      range: &'src ast::IterRange<'src>,
    ) -> Result<()> {
      let [cond, latch, body, end] = self.labels(["cond", "latch", "body", "end"]);
      self.state.builder.allow_unused_label(cond);
      self.state.builder.allow_unused_label(latch);
      self.state.builder.allow_unused_label(body);
      self.state.builder.allow_unused_label(end);

      self.state.begin_loop(latch, end);
      self.state.begin_scope();

      // &item = start
      // <end> = end
      let item = self.reg();
      self
        .state
        .declare_local(stmt.item.deref().clone(), item.clone());
      self.emit_expr(&range.start)?;
      self.emit_op(StoreReg { reg: item.index() });
      let end_v = self.reg();
      self.emit_expr(&range.end)?;
      self.emit_op(StoreReg { reg: end_v.index() });

      // @cond:
      //   if not &item $op <end>:
      //     jump @end
      //   jump @body
      self.finish_label(cond);
      self.emit_op(LoadReg { reg: end_v.index() });
      if range.inclusive {
        self.emit_op(CmpLe { lhs: item.index() });
      } else {
        self.emit_op(CmpLt { lhs: item.index() });
      }
      self.emit_op(JumpIfFalse { offset: end.id() });
      self.emit_op(Jump { offset: body.id() });

      // @latch:
      //   &item += 1
      //   jump @cond
      self.finish_label(latch);
      self.emit_op(PushSmallInt { value: 1 });
      self.emit_op(Add { lhs: item.index() });
      self.emit_op(StoreReg { reg: item.index() });
      self.emit_op(Jump { offset: cond.id() });

      // @body:
      //   <body>
      //   jump @latch
      self.finish_label(body);
      for stmt in stmt.body.iter() {
        // break in <body> = jump @end
        // continue in <body> = jump @latch
        self.emit_stmt(stmt)?;
      }
      self.emit_op(Jump { offset: latch.id() });

      end_v.index();
      item.index();

      // @end:
      self.finish_label(end);
      self.state.end_scope();
      self.state.end_loop();
      Ok(())
    }

    fn emit_for_iter_loop(
      &mut self,
      stmt: &'src ast::For<'src>,
      iter: &'src ast::Expr<'src>,
    ) -> Result<()> {
      todo!()
    }

    fn emit_while_loop(&mut self, stmt: &'src ast::While<'src>) -> Result<()> {
      let [start, end] = self.labels(["start", "end"]);
      self.state.builder.allow_unused_label(start);
      self.state.builder.allow_unused_label(end);

      self.state.begin_loop(start, end);
      self.state.begin_scope();
      self.finish_label(start);

      // condition
      self.emit_expr(&stmt.cond)?;
      self.emit_op(JumpIfFalse { offset: end.id() });
      // body
      for stmt in stmt.body.iter() {
        self.emit_stmt(stmt)?;
      }
      self.emit_op(Jump { offset: start.id() });

      self.finish_label(end);
      self.state.end_scope();
      self.state.end_loop();

      Ok(())
    }

    fn emit_inf_loop(&mut self, stmt: &'src ast::Infinite<'src>) -> Result<()> {
      let [start, end] = self.labels(["start", "end"]);
      self.state.builder.allow_unused_label(start);
      self.state.builder.allow_unused_label(end);

      self.state.begin_loop(start, end);
      self.state.begin_scope();
      self.finish_label(start);

      // body
      for stmt in stmt.body.iter() {
        self.emit_stmt(stmt)?;
      }
      self.emit_op(Jump { offset: start.id() });

      self.finish_label(end);
      self.state.end_scope();
      self.state.end_loop();

      Ok(())
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
        ast::Ctrl::Continue => {
          let loop_ = self
            .state
            .current_loop()
            .expect("attempted to emit continue outside of loop");
          self.emit_op(Jump {
            offset: loop_.start.id(),
          });
        }
        ast::Ctrl::Break => {
          let loop_ = self
            .state
            .current_loop()
            .expect("attempted to emit continue outside of loop");
          self.emit_op(Jump {
            offset: loop_.end.id(),
          });
        }
      }

      Ok(())
    }

    fn emit_func_const(&mut self, stmt: &'src ast::Func<'src>) -> Result<()> {
      let result = self.emit_func(stmt)?;

      if result.captures.is_empty() {
        let func = self.const_value(result.func);
        self.emit_op(LoadConst { slot: func });
      } else {
        let desc = self.const_value(func::ClosureDesc {
          func: result.func,
          num_captures: result.captures.len() as u32,
        });
        self.emit_op(CreateClosure { desc });
        for (_, info) in result.captures.iter() {
          match &info.kind {
            UpvalueKind::Register(reg) => self.emit_op(CaptureReg {
              reg: reg.index(),
              slot: info.slot,
            }),
            UpvalueKind::Capture(slot) => self.emit_op(CaptureSlot {
              parent_slot: *slot,
              self_slot: info.slot,
            }),
          };
        }
      }

      Ok(())
    }

    fn emit_func_stmt(&mut self, stmt: &'src ast::Func<'src>) -> Result<()> {
      let name = stmt.name.deref().clone();

      self.emit_func_const(stmt)?;

      if self.state.is_global_scope() {
        let name = self.const_name(&stmt.name);
        self.emit_op(StoreGlobal { name });
      } else {
        let reg = self.reg();
        self.emit_op(StoreReg { reg: reg.index() });
        self.state.declare_local(name, reg);
      }

      Ok(())
    }

    fn emit_class_stmt(&mut self, stmt: &'src ast::Class<'src>) -> Result<()> {
      let desc = self.const_value(class::ClassDesc {
        name: stmt.name.to_string(),
        params: ast_class_to_params(stmt),
        is_derived: stmt.parent.is_some(),
        methods: stmt.methods.iter().map(|f| f.name.to_string()).collect(),
        fields: stmt.fields.iter().map(|f| f.name.to_string()).collect(),
      });

      // emit parent
      let parent = if let Some(name) = stmt.parent.as_deref() {
        let parent = self.reg();
        match self.resolve_var(name.clone()) {
          Get::Local(reg) => self.emit_op(LoadReg { reg: reg.index() }),
          Get::Capture(slot) => self.emit_op(LoadCapture { slot }),
          Get::Global => {
            let name = self.const_name(name);
            self.emit_op(LoadGlobal { name });
          }
        }
        self.emit_op(StoreReg {
          reg: parent.index(),
        });
        Some(parent)
      } else {
        None
      };

      // allocate registers
      let methods = (0..stmt.methods.len())
        .map(|_| self.reg())
        .collect::<Vec<_>>();
      let fields = (0..stmt.fields.len())
        .map(|_| self.reg())
        .collect::<Vec<_>>();

      // emit methods
      for (method, reg) in stmt.methods.iter().zip(methods.iter()) {
        self.emit_func_const(method)?;
        self.emit_op(StoreReg { reg: reg.index() });
      }

      // emit field defaults
      for (field, value) in stmt.fields.iter().zip(fields.iter()) {
        match &field.default {
          Some(default) => self.emit_expr(default)?,
          None => self.emit_op(PushNone),
        }
        self.emit_op(StoreReg { reg: value.index() });
      }

      // emit create class op
      if let Some(parent) = &parent {
        let start = parent.index();
        self.emit_op(CreateClass { desc, start });
      } else if !methods.is_empty() {
        let start = methods[0].index();
        self.emit_op(CreateClass { desc, start })
      } else if !fields.is_empty() {
        let start = fields[0].index();
        self.emit_op(CreateClass { desc, start })
      } else {
        self.emit_op(CreateClassEmpty { desc });
      }

      let name = stmt.name.deref().clone();
      if self.state.is_global_scope() {
        let name = self.const_name(&name);
        self.emit_op(StoreGlobal { name });
      } else {
        let reg = self.reg();
        self.emit_op(StoreReg { reg: reg.index() });
        self.state.declare_local(name, reg);
      }

      // extend live intervals of methods and fields
      //   - this ensures that regalloc doesn't reallocate our argument registers to
      //     any potential intermediate registers in the argument expressions.
      for v in fields.iter().rev() {
        v.index();
      }
      for m in methods.iter().rev() {
        m.index();
      }
      if let Some(parent) = parent {
        parent.index();
      }

      Ok(())
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
      for symbol in stmt.symbols.iter() {
        let segments = symbol.path.iter().map(|s| s.to_string()).collect();
        let path = self.const_value(module::Path::new(segments));

        let name = match &symbol.alias {
          Some(alias) => alias,
          None => symbol.path.last().unwrap(),
        };
        let dest = self.reg();
        self.state.declare_local(name.deref().clone(), dest.clone());

        self.emit_op(LoadModule {
          path,
          dest: dest.index(),
        });
      }

      Ok(())
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
        ast::ExprKind::GetField(v) => self.emit_get_field_expr(v),
        ast::ExprKind::SetField(v) => self.emit_set_field_expr(v),
        ast::ExprKind::GetIndex(v) => self.emit_get_index_expr(v),
        ast::ExprKind::SetIndex(v) => self.emit_set_index_expr(v),
        ast::ExprKind::Yield(v) => self.emit_yield_expr(v),
        ast::ExprKind::Call(v) => self.emit_call_expr(v),
        ast::ExprKind::GetSelf => self.emit_get_self_expr(),
        ast::ExprKind::GetSuper => self.emit_get_super_expr(),
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
            // TODO: use `InsertToDictNamed for constant keys`
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
          let [lhs, end] = self.labels(["lhs", "end"]);
          let reg = self.reg();
          self.emit_expr(&expr.left)?;
          self.emit_op(StoreReg { reg: reg.index() });
          self.emit_op(IsNone);
          self.emit_op(JumpIfFalse { offset: lhs.id() });
          self.emit_expr(&expr.right)?;
          self.emit_op(Jump { offset: end.id() });
          self.finish_label(lhs);
          self.emit_op(LoadReg { reg: reg.index() });
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

    fn emit_get_field_expr(&mut self, expr: &'src ast::GetField<'src>) -> Result<()> {
      let name = self.const_name(&expr.name);
      self.emit_expr(&expr.target)?;
      if self.state.is_in_opt_expr {
        self.emit_op(LoadFieldOpt { name });
      } else {
        self.emit_op(LoadField { name });
      }

      Ok(())
    }

    fn emit_set_field_expr(&mut self, expr: &'src ast::SetField<'src>) -> Result<()> {
      let obj = self.reg();
      let name = self.const_name(&expr.target.name);
      self.emit_expr(&expr.target.target)?;
      self.emit_op(StoreReg { reg: obj.index() });
      self.emit_expr(&expr.value)?;
      self.emit_op(StoreField {
        name,
        obj: obj.index(),
      });

      Ok(())
    }

    fn emit_get_index_expr(&mut self, expr: &'src ast::GetIndex<'src>) -> Result<()> {
      let key = self.reg();
      self.emit_expr(&expr.key)?;
      self.emit_op(StoreReg { reg: key.index() });
      self.emit_expr(&expr.target)?;
      if self.state.is_in_opt_expr {
        self.emit_op(LoadIndexOpt { key: key.index() });
      } else {
        self.emit_op(LoadIndex { key: key.index() });
      }

      Ok(())
    }

    fn emit_set_index_expr(&mut self, expr: &'src ast::SetIndex<'src>) -> Result<()> {
      let obj = self.reg();
      let key = self.reg();
      self.emit_expr(&expr.target.key)?;
      self.emit_op(StoreReg { reg: key.index() });
      self.emit_expr(&expr.target.target)?;
      self.emit_op(StoreReg { reg: obj.index() });
      self.emit_expr(&expr.value)?;
      self.emit_op(StoreIndex {
        key: key.index(),
        obj: obj.index(),
      });

      Ok(())
    }

    fn emit_yield_expr(&mut self, expr: &'src ast::Yield<'src>) -> Result<()> {
      todo!()
    }

    fn emit_call_expr(&mut self, expr: &'src ast::Call<'src>) -> Result<()> {
      // 1. emit args (reg)
      // 2. emit kw dict (reg)
      // 3. emit callee (acc)
      // 4. emit op Call (#args)

      // allocate registers
      let kw = if !expr.args.kw.is_empty() {
        Some(self.reg())
      } else {
        None
      };
      let argv = (0..expr.args.pos.len())
        .map(|_| self.reg())
        .collect::<Vec<_>>();

      // emit args
      for (reg, value) in argv.iter().zip(expr.args.pos.iter()) {
        self.emit_expr(value)?;
        self.emit_op(StoreReg { reg: reg.index() });
      }

      // emit kw dict
      if let Some(kw) = &kw {
        self.emit_op(CreateEmptyDict);
        self.emit_op(StoreReg { reg: kw.index() });
        for (name, value) in expr.args.kw.iter() {
          let name = self.const_name(name);
          self.emit_expr(value)?;
          self.emit_op(InsertToDictNamed {
            name,
            dict: kw.index(),
          });
        }
      }

      // emit callee
      self.emit_expr(&expr.target)?;

      // extend live intervals of args
      //   - this ensures that regalloc doesn't reallocate our argument registers to
      //     any potential intermediate registers in the argument expressions.
      for r in argv.iter().skip(1).rev() {
        r.index();
      }
      let arg0 = argv.first().map(|r| r.index());
      let kw = kw.map(|r| r.index());
      let args = argv.len() as u32;

      // emit op
      let op: Instruction = match (kw, arg0) {
        (Some(kw), _) => CallKw { start: kw, args }.into(),
        (None, Some(arg0)) => Call { start: arg0, args }.into(),
        _ => Call0.into(),
      };
      self.emit_op(op);

      Ok(())
    }

    fn emit_get_self_expr(&mut self) -> Result<()> {
      self.emit_op(LoadSelf);

      Ok(())
    }

    fn emit_get_super_expr(&mut self) -> Result<()> {
      self.emit_op(LoadSuper);

      Ok(())
    }
  }
}

#[cfg(test)]
mod tests;
