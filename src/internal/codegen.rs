// TEMP
#![allow(dead_code)]

mod expr;
mod regalloc;
mod stmt;

use indexmap::{IndexMap, IndexSet};

use self::regalloc::{RegAlloc, Register, Slice};
use super::bytecode::builder::{BytecodeBuilder, InsertConstant, LoopHeader, MultiLabel};
use super::bytecode::opcode::symbolic::*;
use super::bytecode::opcode::{self as op};
use super::object;
use super::object::function;
use super::object::ptr::Ptr;
use super::syntax::ast;
use super::vm::global::Global;
use crate::span::Span;
use crate::Cow;

pub fn emit<'src>(
  global: Global,
  ast: &'src ast::Module<'src>,
  name: impl Into<Cow<'src, str>>,
  is_root: bool,
) -> Ptr<object::ModuleDescriptor> {
  let name = name.into();

  let mut module = State::new(global.clone(), ast, name.clone(), is_root).emit_module();

  let name = global.alloc(object::Str::owned(name));
  // NOTE: no need to handle `.upvalues` here,
  // because the module root never has any upvalues
  let root = module.functions.pop().unwrap().finish().ptr;
  let module_vars = module.vars;

  global.alloc(object::ModuleDescriptor {
    name,
    root,
    module_vars,
  })
}

struct State<'src> {
  global: Global,
  ast: &'src ast::Module<'src>,
  module: Module<'src>,
}

impl<'src> State<'src> {
  fn new(
    global: Global,
    ast: &'src ast::Module<'src>,
    name: impl Into<Cow<'src, str>>,
    is_root: bool,
  ) -> Self {
    Self {
      global: global.clone(),
      ast,
      module: Module {
        is_root,
        vars: IndexSet::new(),
        functions: vec![Function::new(
          global,
          name,
          function::Params::default(),
          false,
          false,
        )],
      },
    }
  }

  fn current_function(&mut self) -> &mut Function<'src> {
    self.module.functions.last_mut().unwrap()
  }

  fn builder(&mut self) -> &mut BytecodeBuilder {
    &mut self.current_function().builder
  }

  fn is_global_scope(&self) -> bool {
    self.module.functions.len() <= 1
  }

  fn constant_name(&mut self, string: impl ToString) -> op::Constant {
    let string = self.global.intern(string.to_string());
    self.builder().constant_pool_builder().insert(string)
  }

  fn constant_value(&mut self, value: impl InsertConstant) -> op::Constant {
    self.builder().constant_pool_builder().insert(value)
  }

  fn alloc_register(&mut self) -> Register {
    self.current_function().regalloc.alloc()
  }

  fn alloc_register_slice(&mut self, n: usize) -> Slice {
    self.current_function().regalloc.alloc_slice(n)
  }

  fn emit_var(&mut self, name: impl Into<Cow<'src, str>>, span: Span) {
    let name = name.into();
    if self.is_global_scope() {
      if self.module.is_root {
        let name = self.constant_name(name);
        self.builder().emit(StoreGlobal { name }, span);
      } else {
        let idx = self.declare_module_var(name);
        self.builder().emit(StoreModuleVar { idx }, span);
      }
    } else {
      let register = self.alloc_register();
      self.emit_store(register.clone(), span);
      self.declare_local(name, register);
    }
  }

  fn declare_local(&mut self, name: impl Into<Cow<'src, str>>, register: Register) {
    let function = self.current_function();

    let _ = register.access(); // ensure liveness at time of declaration
    let name = name.into();

    let key = (function.scope, name);
    let existing = function.locals.insert(key, register.clone());

    if let Some(local) = existing {
      let _ = local.access();
      local.ensure_non_overlapping(register);
    }
  }

  fn declare_module_var(&mut self, name: impl Into<Cow<'src, str>>) -> op::ModuleVar {
    let name = self.global.intern(name.into().to_string());
    let index = self.module.vars.len() as u32;
    self.module.vars.insert(name);
    op::ModuleVar(index)
  }

  fn resolve_var(&mut self, name: impl Into<Cow<'src, str>>) -> Get {
    let name = name.into();

    if let Some(reg) = self.current_function().resolve_local(&name) {
      return Get::Local(reg);
    }

    if let Some(reg) = self.resolve_upvalue(&name, self.module.functions.len() - 1) {
      return Get::Upvalue(reg);
    }

    if let Some(reg) = self.resolve_module_var(&name) {
      return Get::ModuleVar(reg);
    }

    Get::Global
  }

  fn resolve_upvalue(
    &mut self,
    name: &Cow<'src, str>,
    function_index: usize,
  ) -> Option<op::Upvalue> {
    if function_index < 1 {
      return None;
    }

    if let Some(info) = self.module.functions[function_index].upvalues.get(name) {
      return Some(info.dst);
    }

    let local_slot = op::Upvalue(self.module.functions[function_index].upvalues.len() as u32);
    if let Some(reg) = self.module.functions[function_index - 1].resolve_local(name) {
      self.module.functions[function_index].upvalues.insert(
        name.clone(),
        Upvalue {
          dst: local_slot,
          src: UpvalueSource::Register(reg),
        },
      );
      return Some(local_slot);
    }

    if let Some(parent_slot) = self.resolve_upvalue(name, function_index - 1) {
      self.module.functions[function_index].upvalues.insert(
        name.clone(),
        Upvalue {
          dst: local_slot,
          src: UpvalueSource::Upvalue(parent_slot),
        },
      );
      return Some(local_slot);
    }

    None
  }

  fn resolve_module_var(&self, name: impl AsRef<str>) -> Option<op::ModuleVar> {
    self
      .module
      .vars
      .get_index_of(name.as_ref())
      .map(|v| op::ModuleVar(v as u32))
  }

  fn emit_get(&mut self, name: impl Into<Cow<'src, str>>, span: Span) {
    let name = name.into();
    match self.resolve_var(name.clone()) {
      Get::Local(reg) => self.builder().emit(Load { reg: reg.access() }, span),
      Get::Upvalue(idx) => self.builder().emit(LoadUpvalue { idx }, span),
      Get::ModuleVar(idx) => self.builder().emit(LoadModuleVar { idx }, span),
      Get::Global => {
        let name = self.constant_name(name);
        self.builder().emit(LoadGlobal { name }, span)
      }
    }
  }

  #[inline]
  fn emit_store(&mut self, register: Register, span: Span) {
    self.builder().emit(
      Store {
        reg: register.access(),
      },
      span,
    )
  }

  #[inline]
  fn emit_load(&mut self, register: Register, span: Span) {
    self.builder().emit(
      Load {
        reg: register.access(),
      },
      span,
    );
  }

  fn emit_function(&mut self, func: &'src ast::Func<'src>, is_init: bool) -> EmittedFunction<'src> {
    self.module.functions.push(Function::new(
      self.global.clone(),
      func.name.lexeme(),
      function::Params::from_ast_func(func),
      func.has_yield,
      is_init,
    ));

    self.current_function().enter_scope();

    // allocate registers
    let param_slice = self.alloc_register_slice(1 + func.params.pos.len());
    let (callee, receiver, positional) = match func.params.has_self {
      true => (None, Some(param_slice.get(0)), param_slice.offset(1)),
      false => (Some(param_slice.get(0)), None, param_slice.offset(1)),
    };

    // declare function and receiver
    // the function param only exists for plain functions,
    // and the receiver only exists for methods.
    //
    // the point of this is to give access to:
    // - `self` in methods.
    // - the function being called in recursive functions.
    if let Some(callee) = &callee {
      self.declare_local(func.name.lexeme(), callee.clone());
    }
    if let Some(receiver) = &receiver {
      self.declare_local("self", receiver.clone());
    }

    // emit default values
    for (i, param) in func.params.pos.iter().enumerate() {
      if let Some(default) = &param.default {
        let next = self.builder().label("next");
        self.emit_load(positional.get(i), param.span());
        self.builder().emit(IsNone, param.span());
        self.builder().emit_jump_if_false(&next, param.span());
        self.emit_expr(default);
        self.emit_store(positional.get(i), param.span());
        self.builder().bind_label(next);
      }
    }

    // declare parameters
    // this happens *after* emitting the defaults, because the
    // defaults should not be able to access the parameters
    for (i, param) in func.params.pos.iter().enumerate() {
      self.declare_local(param.name.lexeme(), positional.get(i));
    }

    // emit body
    for stmt in func.body.iter() {
      self.emit_stmt(stmt);
    }

    // all functions return `none` by default
    let end_span = func
      .body
      .last()
      .map(|stmt| stmt.span)
      .unwrap_or((0..0).into());

    if self.current_function().is_init {
      self.builder().emit(LoadSelf, end_span);
    } else {
      self.builder().emit(LoadNone, end_span);
    }
    self.builder().emit(Return, end_span);

    self.current_function().leave_scope();

    let function = self.module.functions.pop().unwrap().finish();

    self
      .current_function()
      .inner_functions
      .push(function.ptr.clone());

    function
  }

  fn emit_module(mut self) -> Module<'src> {
    let callee = self.alloc_register();
    self.current_function().enter_scope();
    for stmt in self.ast.body.iter() {
      self.emit_stmt(stmt);
    }
    self.current_function().leave_scope();
    if !self.module.is_root {
      self.builder().emit(FinalizeModule, 0..0);
    }
    self.builder().emit(Return, 0..0);
    let _ = callee.access();

    self.module
  }
}

impl function::Params {
  pub fn from_ast_func(func: &ast::Func) -> Self {
    let mut min = 0;
    let mut max = 0;
    for param in func.params.pos.iter() {
      if param.default.is_none() {
        min += 1;
      }
      max += 1;
    }

    Self {
      has_self: func.params.has_self,
      min,
      max,
    }
  }
}

struct Module<'src> {
  is_root: bool,
  vars: IndexSet<Ptr<object::Str>>,
  functions: Vec<Function<'src>>,
}

struct EmittedFunction<'src> {
  ptr: Ptr<object::FunctionDescriptor>,
  upvalues: Upvalues<'src>,
}

struct Function<'src> {
  global: Global,

  // TODO: these two should be mutually exclusive
  is_generator: bool,
  is_init: bool,

  name: Cow<'src, str>,
  builder: BytecodeBuilder,
  regalloc: RegAlloc,

  params: function::Params,
  locals: IndexMap<(Scope, Cow<'src, str>), Register>,
  upvalues: IndexMap<Cow<'src, str>, Upvalue>,
  scope: Scope,

  is_in_opt_expr: bool,
  current_loop: Option<Loop>,

  inner_functions: Vec<Ptr<object::FunctionDescriptor>>,
}

impl<'src> Function<'src> {
  fn new(
    global: Global,
    name: impl Into<Cow<'src, str>>,
    params: function::Params,
    is_generator: bool,
    is_init: bool,
  ) -> Self {
    Self {
      global,

      is_generator,
      is_init,

      name: name.into(),
      builder: BytecodeBuilder::new(),
      regalloc: RegAlloc::new(),

      params,
      locals: IndexMap::new(),
      upvalues: IndexMap::new(),

      scope: Scope(0),

      is_in_opt_expr: false,
      current_loop: None,

      inner_functions: Vec::new(),
    }
  }

  fn enter_scope(&mut self) {
    self.scope.0 += 1;
  }

  fn leave_scope(&mut self) {
    let current_scope = self.scope;
    self.locals.retain(|(scope, _), register| {
      let retain = *scope != current_scope;
      if !retain {
        let _ = register.access();
      }
      retain
    });
    self.scope.0 -= 1;
  }

  fn enter_loop_body(&mut self, start: LoopHeader, end: MultiLabel) -> Option<Loop> {
    self.current_loop.replace(Loop { start, end })
  }

  fn leave_loop_body(&mut self, previous: Option<Loop>) -> Loop {
    let current = self.current_loop.take().unwrap();
    if let Some(previous) = previous {
      self.current_loop = Some(previous);
    }
    current
  }

  fn resolve_local(&self, name: &Cow<'src, str>) -> Option<Register> {
    self
      .locals
      .iter()
      .rev()
      .find(|((_, var), _)| var == name)
      .map(|(_, register)| register.clone())
  }

  fn finish(self) -> EmittedFunction<'src> {
    let (frame_size, register_map) = self.regalloc.finish();
    let (mut bytecode, constants) = self.builder.finish();

    // patch registers in bytecode
    op::patch_registers(&mut bytecode, &register_map);

    // patch registers in inner functions
    for function in self.inner_functions.iter() {
      for upvalue in function.upvalues.borrow_mut().iter_mut() {
        match upvalue {
          function::Upvalue::Register(register) => {
            *register = op::Register(register_map[register.0 as usize] as u32)
          }
          function::Upvalue::Upvalue(_) => {}
        }
      }
    }

    let ptr = self.global.alloc(object::FunctionDescriptor::new(
      self.global.intern(self.name.to_string()),
      self.is_generator,
      self.params,
      self
        .upvalues
        .values()
        .map(|v| match &v.src {
          UpvalueSource::Register(register) => function::Upvalue::Register(register.access()),
          UpvalueSource::Upvalue(index) => function::Upvalue::Upvalue(*index),
        })
        .collect(),
      frame_size,
      bytecode,
      constants,
    ));
    let upvalues = Upvalues(self.upvalues);

    EmittedFunction { ptr, upvalues }
  }
}

struct Upvalues<'src>(IndexMap<Cow<'src, str>, Upvalue>);

impl<'src> Upvalues<'src> {
  fn finish(&self) {
    for upvalue in self.0.values() {
      if let UpvalueSource::Register(register) = &upvalue.src {
        let _ = register.access();
      }
    }
  }
}

struct Upvalue {
  dst: op::Upvalue,
  src: UpvalueSource,
}

enum UpvalueSource {
  Register(Register),
  Upvalue(op::Upvalue),
}

enum Get {
  Local(Register),
  Upvalue(op::Upvalue),
  ModuleVar(op::ModuleVar),
  Global,
}

struct Loop {
  start: LoopHeader,
  end: MultiLabel,
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Scope(usize);

#[cfg(all(test, not(feature = "__miri")))]
mod tests;
