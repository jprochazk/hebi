// TEMP
#![allow(dead_code)]

mod expr;
mod regalloc;
mod stmt;

// TODO:
// - (optimization) constant pool compaction
// - (optimization) basic blocks
// - (optimization) elide last instruction (clobbered read)
// - (optimization) peephole with last 2 instructions
// - actually write emit for all AST nodes

use beef::lean::Cow;
use indexmap::{IndexMap, IndexSet};

use self::regalloc::{RegAlloc, Register, Slice};
use crate::bytecode::builder::{BytecodeBuilder, InsertConstant, LoopHeader, MultiLabel};
use crate::bytecode::opcode::symbolic::*;
use crate::bytecode::opcode::{self as op};
use crate::ctx::Context;
use crate::span::Span;
use crate::syntax::ast;
use crate::value::object;
use crate::value::object::function;
use crate::value::object::ptr::Ptr;

pub fn emit<'cx, 'src>(
  cx: &'cx Context,
  ast: &'src ast::Module<'src>,
  name: impl Into<Cow<'src, str>>,
  is_root: bool,
) -> Ptr<object::ModuleDescriptor> {
  let name = name.into();

  let mut module = State::new(cx, ast, name.clone(), is_root).emit_module();

  let name = cx.alloc(object::String::new(name.to_string().into()));
  let root = module.functions.pop().unwrap().finish();
  let module_vars = module.vars;

  cx.alloc(object::ModuleDescriptor {
    name,
    root,
    module_vars,
  })
}

struct State<'cx, 'src> {
  cx: &'cx Context,
  ast: &'src ast::Module<'src>,
  module: Module<'cx, 'src>,
}

impl<'cx, 'src> State<'cx, 'src> {
  fn new(
    cx: &'cx Context,
    ast: &'src ast::Module<'src>,
    name: impl Into<Cow<'src, str>>,
    is_root: bool,
  ) -> Self {
    Self {
      cx,
      ast,
      module: Module {
        is_root,
        vars: IndexSet::new(),
        functions: vec![Function::new(cx, name, function::Params::default())],
      },
    }
  }

  fn current_function(&mut self) -> &mut Function<'cx, 'src> {
    self.module.functions.last_mut().unwrap()
  }

  fn builder(&mut self) -> &mut BytecodeBuilder {
    &mut self.current_function().builder
  }

  fn is_global_scope(&self) -> bool {
    self.module.functions.len() <= 1
  }

  fn constant_name(&mut self, string: impl ToString) -> op::Constant {
    let string = self.cx.intern(string.to_string());
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
      self.builder().emit(
        Store {
          reg: register.access(),
        },
        span,
      );
      self.declare_local(name, register);
    }
  }

  fn declare_local(&mut self, name: impl Into<Cow<'src, str>>, register: Register) {
    let function = self.current_function();

    let _ = register.access(); // ensure liveness at time of declaration
    let name = name.into();

    let key = (function.scope, name);
    if let Some(var) = function.locals.get_mut(&key) {
      *var = register;
    } else {
      function.locals.insert(key, register);
    }
  }

  fn declare_module_var(&mut self, name: impl Into<Cow<'src, str>>) -> op::ModuleVar {
    let name = self.cx.intern(name.into().to_string());
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
    if function_index < 2 {
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

  fn emit_function(
    &mut self,
    name: impl Into<Cow<'src, str>>,
    has_self: bool,
    params: &'src [ast::Param<'src>],
    body: &'src [ast::Stmt<'src>],
  ) -> Ptr<object::FunctionDescriptor> {
    let name = name.into();

    self.module.functions.push(Function::new(
      self.cx,
      name.clone(),
      function::Params::from_ast(has_self, params),
    ));

    // allocate registers
    let param_slice = self.alloc_register_slice(1 + params.len());
    let (func, receiver, positional) = match has_self {
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
    if let Some(func) = &func {
      self.declare_local(name.clone(), func.clone());
    }
    if let Some(receiver) = &receiver {
      self.declare_local("self", receiver.clone());
    }

    // emit default values
    for (i, param) in params.iter().enumerate() {
      if let Some(default) = &param.default {
        let next = self.builder().label("next");
        self.builder().emit(
          Load {
            reg: positional.access(i),
          },
          param.span(),
        );
        self.builder().emit(IsNone, param.span());
        self.builder().emit_jump_if_false(&next, param.span());
        self.emit_expr(default);
        self.builder().emit(
          Store {
            reg: positional.access(i),
          },
          param.span(),
        );
        self.builder().bind_label(next);
      }
    }

    // declare parameters
    // this happens *after* emitting the defaults, because the
    // defaults should not be able to access the parameters
    for (i, param) in params.iter().enumerate() {
      self.declare_local(param.name.lexeme(), positional.get(i));
    }

    // emit body
    for stmt in body.iter() {
      self.emit_stmt(stmt);
    }

    // all functions return `none` by default
    // TODO: only emit this if `exit_seen` is false
    let end_span = body.last().map(|stmt| stmt.span).unwrap_or((0..0).into());
    self.builder().emit(LoadNone, end_span);
    self.builder().emit(Ret, end_span);

    self.module.functions.pop().unwrap().finish()
  }

  fn emit_module(mut self) -> Module<'cx, 'src> {
    for stmt in self.ast.body.iter() {
      self.emit_stmt(stmt);
    }
    self.builder().emit(Ret, 0..0);

    self.module
  }
}

impl function::Params {
  pub fn from_ast(has_self: bool, params: &[ast::Param]) -> Self {
    let mut min = 0;
    let mut max = 0;
    for param in params {
      if param.default.is_none() {
        min += 1;
      }
      max += 1;
    }

    Self { has_self, min, max }
  }
}

struct Module<'cx, 'src> {
  is_root: bool,
  vars: IndexSet<Ptr<object::String>>,
  functions: Vec<Function<'cx, 'src>>,
}

struct Function<'cx, 'src> {
  cx: &'cx Context,

  name: Cow<'src, str>,
  builder: BytecodeBuilder,
  regalloc: RegAlloc,

  params: function::Params,
  locals: IndexMap<(Scope, Cow<'src, str>), Register>,
  upvalues: IndexMap<Cow<'src, str>, Upvalue>,
  scope: Scope,

  is_in_opt_expr: bool,
  current_loop: Option<Loop>,
}

impl<'cx, 'src> Function<'cx, 'src> {
  fn new(cx: &'cx Context, name: impl Into<Cow<'src, str>>, params: function::Params) -> Self {
    Self {
      cx,

      name: name.into(),
      builder: BytecodeBuilder::new(),
      regalloc: RegAlloc::new(),

      params,
      locals: IndexMap::new(),
      upvalues: IndexMap::new(),

      scope: Scope(0),

      is_in_opt_expr: false,
      current_loop: None,
    }
  }

  fn enter_scope(&mut self) {
    self.scope.0 += 1;
  }

  fn leave_scope(&mut self) {
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

  fn finish(self) -> Ptr<object::FunctionDescriptor> {
    let (frame_size, register_map) = self.regalloc.finish();
    let (mut bytecode, constants) = self.builder.finish();
    op::patch_registers(&mut bytecode, &register_map);

    self.cx.alloc(object::FunctionDescriptor::new(
      self
        .cx
        .alloc(object::String::new(self.name.to_string().into())),
      self.params,
      self.upvalues.len(),
      frame_size,
      bytecode,
      constants,
    ))
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

#[cfg(test)]
mod tests;
