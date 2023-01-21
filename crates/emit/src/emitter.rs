use std::collections::HashMap;
use std::ops::Deref;

use beef::lean::Cow;
use op::instruction::*;
use syntax::ast;
use value::object::func;
use value::Value;

pub fn emit<'src>(
  name: impl Into<Cow<'src, str>>,
  module: &'src ast::Module<'src>,
) -> Result<func::Func> {
  Emitter::new(name, module).emit_main()
}

use crate::regalloc::{RegAlloc, Register};
use crate::{Error, Result};

struct Emitter<'src> {
  state: Function<'src>,
  module: &'src ast::Module<'src>,
  /// String interning
  strings: HashMap<String, u32>,
}

impl<'src> Emitter<'src> {
  fn new(name: impl Into<Cow<'src, str>>, module: &'src ast::Module<'src>) -> Self {
    Self {
      state: Function::new(name, None),
      module,
      strings: HashMap::new(),
    }
  }

  fn emit_main(mut self) -> Result<func::Func> {
    for stmt in self.module.body.iter() {
      self.emit_stmt(stmt)?;
    }
    self.emit_op(Ret);

    let (frame_size, register_map) = self.state.regalloc.scan();

    self
      .state
      .builder
      .patch(|instructions| patch_registers(instructions, &register_map));

    let Chunk {
      name,
      bytecode,
      const_pool,
    } = self.state.builder.build();

    Ok(func::Func::new(
      name,
      frame_size,
      bytecode,
      const_pool,
      func::Params {
        min: 0,
        max: None,
        kw: Default::default(),
      },
    ))
  }

  fn emit_func(
    &mut self,
    name: impl Into<Cow<'src, str>>,
    params: func::Params,
    f: impl FnOnce(&mut Self) -> Result<()>,
  ) -> Result<func::Func> {
    let next = Function::new(name.into(), None);
    let parent = std::mem::replace(&mut self.state, next);
    self.state.parent = Some(Box::new(parent));

    let result = f(self);

    let parent = self
      .state
      .parent
      .take()
      .expect("`self.state.parent` was set to `None` inside of callback passed to `emit_chunk`");
    let mut next = std::mem::replace(&mut self.state, *parent);

    result?;

    let (frame_size, register_map) = next.regalloc.scan();

    next
      .builder
      .patch(|instructions| patch_registers(instructions, &register_map));

    let Chunk {
      name,
      bytecode,
      const_pool,
    } = next.builder.build();

    Ok(func::Func::new(
      name, frame_size, bytecode, const_pool, params,
    ))
  }

  fn const_(&mut self, value: impl Into<Value>) -> u32 {
    let value: Value = value.into();

    // TODO: global string interner, this only interns locally
    // which means that "a" from module != "a" from a different module
    // intern strings
    if value.is_string() {
      let str = value.as_string().unwrap();
      if let Some(slot) = self.strings.get(str.as_str()).cloned() {
        return slot;
      }

      let slot = self.state.builder.constant(value.clone());
      self.strings.insert(str.clone(), slot);
      return slot;
    }

    self.state.builder.constant(value)
  }

  fn emit_op(&mut self, op: impl Into<Instruction>) {
    self.state.builder.op(op)
  }

  fn reg(&mut self) -> Register {
    self.state.regalloc.alloc()
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

struct Capture {
  /// Capture slot
  slot: u32,
  /// Captured register from enclosing scope
  ///
  /// Used to emit `Capture` instructions after emitting closures
  register: Register,
}

struct Function<'src> {
  builder: Builder<Value>,
  name: Cow<'src, str>,
  parent: Option<Box<Function<'src>>>,
  regalloc: RegAlloc,

  /// Map of variable name to register.
  ///
  /// Because locals may shadow other locals, the register is actually a stack
  /// of registers, and only the last one is active.
  ///
  /// Invariants:
  /// - Register stacks may not be empty. If a stack is about to be emptied, the
  ///   local should be removed instead.
  locals: HashMap<Cow<'src, str>, Vec<Register>>,
  /// List of variables captured from an enclosing scope.
  ///
  /// These may be shadowed by a local.
  captures: HashMap<Cow<'src, str>, Capture>,
  capture_slot: u32,
}

impl<'src> Function<'src> {
  fn new(name: impl Into<Cow<'src, str>>, parent: Option<Box<Function<'src>>>) -> Self {
    let name = name.into();
    Self {
      builder: Builder::new(name.to_string()),
      name,
      parent,
      regalloc: RegAlloc::new(),
      locals: HashMap::new(),
      captures: HashMap::new(),
      capture_slot: 0,
    }
  }

  // TODO: remove variables at the end of a block

  fn declare_local(&mut self, name: impl Into<Cow<'src, str>>, reg: Register) {
    let name = name.into();
    if let Some(stack) = self.locals.get_mut(&name) {
      stack.push(reg);
    } else {
      self.locals.insert(name, vec![reg]);
    }
  }

  fn local(&self, name: &str) -> Option<Register> {
    let Some(stack) = self.locals.get(name) else {
      return None;
    };
    let Some(reg) = stack.last() else {
      panic!("local {name} register stack is empty");
    };
    Some(reg.clone())
  }

  fn capture(&mut self, name: impl Into<Cow<'src, str>>) -> Option<u32> {
    let name = name.into();
    let Some(parent) = self.parent.as_deref_mut() else {
      return None;
    };
    let Some(reg) = parent.local(&name) else {
      return parent.capture(name);
    };

    let capture = self.captures.entry(name).or_insert_with(|| {
      let slot = self.capture_slot;
      self.capture_slot += 1;
      Capture {
        slot,
        register: reg,
      }
    });

    Some(capture.slot)
  }
}

fn patch_registers(instructions: &mut Vec<Instruction>, register_map: &HashMap<u32, u32>) {
  // TODO: some kind of trait that is automatically implemented by `instructions!`
  // macro
  for instruction in instructions.iter_mut() {
    match instruction {
      Instruction::Nop(_) => {}
      Instruction::LoadConst(_) => {}
      Instruction::LoadReg(v) => v.reg = register_map[&v.reg],
      Instruction::StoreReg(v) => v.reg = register_map[&v.reg],
      Instruction::LoadCapture(_) => {}
      Instruction::StoreCapture(_) => {}
      Instruction::LoadGlobal(_) => {}
      Instruction::StoreGlobal(_) => {}
      Instruction::LoadNamed(_) => {}
      Instruction::StoreNamed(v) => v.obj = register_map[&v.obj],
      Instruction::LoadKeyed(v) => v.key = register_map[&v.key],
      Instruction::StoreKeyed(v) => {
        v.key = register_map[&v.key];
        v.obj = register_map[&v.obj];
      }
      Instruction::PushNone(_) => {}
      Instruction::PushTrue(_) => {}
      Instruction::PushFalse(_) => {}
      Instruction::PushSmallInt(_) => {}
      Instruction::CreateEmptyList(_) => {}
      Instruction::PushToList(v) => v.list = register_map[&v.list],
      Instruction::CreateEmptyDict(_) => {}
      Instruction::InsertToDict(v) => {
        v.key = register_map[&v.key];
        v.dict = register_map[&v.dict];
      }
      Instruction::InsertToDictKeyed(v) => {
        v.dict = register_map[&v.dict];
      }
      Instruction::Jump(_) => {}
      Instruction::JumpBack(_) => {}
      Instruction::JumpIfFalse(_) => {}
      Instruction::Add(v) => v.lhs = register_map[&v.lhs],
      Instruction::Sub(v) => v.lhs = register_map[&v.lhs],
      Instruction::Mul(v) => v.lhs = register_map[&v.lhs],
      Instruction::Div(v) => v.lhs = register_map[&v.lhs],
      Instruction::Rem(v) => v.lhs = register_map[&v.lhs],
      Instruction::Pow(v) => v.lhs = register_map[&v.lhs],
      Instruction::UnaryPlus(_) => {}
      Instruction::UnaryMinus(_) => {}
      Instruction::UnaryNot(_) => {}
      Instruction::CmpEq(v) => v.lhs = register_map[&v.lhs],
      Instruction::CmpNeq(v) => v.lhs = register_map[&v.lhs],
      Instruction::CmpGt(v) => v.lhs = register_map[&v.lhs],
      Instruction::CmpGe(v) => v.lhs = register_map[&v.lhs],
      Instruction::CmpLt(v) => v.lhs = register_map[&v.lhs],
      Instruction::CmpLe(v) => v.lhs = register_map[&v.lhs],
      Instruction::Print(_) => {}
      Instruction::PrintList(v) => v.list = register_map[&v.list],
      Instruction::Call(v) => v.callee = register_map[&v.callee],
      Instruction::CallKw(v) => v.callee = register_map[&v.callee],
      Instruction::Ret(_) => {}
      Instruction::Suspend(_) => {}
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

    fn emit_var_stmt(&mut self, stmt: &'src ast::Var<'src>) -> Result<()> {
      self.emit_expr(&stmt.value)?;
      if self.state.parent.is_none() {
        let name = self.const_(stmt.name.as_ref());
        self.emit_op(StoreGlobal { name });
      } else {
        let reg = self.reg();
        self.emit_op(StoreReg { reg: reg.index() });
        self.state.declare_local(stmt.name.deref().clone(), reg);
      }
      Ok(())
    }

    fn emit_if_stmt(&mut self, stmt: &'src ast::If<'src>) -> Result<()> {
      todo!()
    }

    fn emit_loop_stmt(&mut self, stmt: &'src ast::Loop<'src>) -> Result<()> {
      todo!()
    }

    fn emit_ctrl_stmt(&mut self, stmt: &'src ast::Ctrl<'src>) -> Result<()> {
      todo!()
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
          .map(|v| String::from(v.0.as_ref()))
          .collect(),
      };

      let func = self.emit_func(name.clone(), params, |this| {
        // TODO: handle params (defaults, `argv`, kw, `kwargs`)

        for stmt in stmt.body.iter() {
          this.emit_stmt(stmt)?;
        }
        this.emit_op(Ret);
        Ok(())
      })?;

      // TODO: emit closure

      let func = self.const_(func);
      self.emit_op(LoadConst { slot: func });
      if self.state.parent.is_none() {
        let name = self.const_(name);
        self.emit_op(StoreGlobal { name });
      } else {
        let reg = self.reg();
        self.emit_op(StoreReg { reg: reg.index() });
        self.state.declare_local(name, reg);
      }

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
          // float is 64 bits so cannot be stored inline,
          // but it is interned
          let num = self.const_(*v);
          self.emit_op(LoadConst { slot: num });
        }
        ast::Literal::Bool(v) => match v {
          true => self.emit_op(PushTrue),
          false => self.emit_op(PushFalse),
        },
        ast::Literal::String(v) => {
          // `const_` interns the string
          let str = self.const_(v.clone());
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
        ast::BinaryOp::And => todo!("short-circuiting and"),
        ast::BinaryOp::Or => todo!("short-circuiting or"),
        ast::BinaryOp::Maybe => todo!("short-circuiting maybe"),
      }

      Ok(())
    }

    fn emit_unary_expr(&mut self, expr: &'src ast::Unary<'src>) -> Result<()> {
      // unary expressions only use the accumulator

      self.emit_expr(&expr.right)?;

      match expr.op {
        ast::UnaryOp::Plus => self.emit_op(UnaryPlus),
        ast::UnaryOp::Minus => self.emit_op(UnaryMinus),
        ast::UnaryOp::Not => self.emit_op(UnaryNot),
        ast::UnaryOp::Opt => todo!("optional access"),
      }

      Ok(())
    }

    fn emit_get_var_expr(&mut self, expr: &'src ast::GetVar<'src>) -> Result<()> {
      match self.resolve_var(expr.name.deref().clone()) {
        Get::Local(reg) => self.emit_op(LoadReg { reg: reg.index() }),
        Get::Capture(slot) => self.emit_op(LoadCapture { slot }),
        Get::Global => {
          let name = self.const_(expr.name.deref().clone());
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
          let name = self.const_(expr.target.name.deref().clone());
          self.emit_op(StoreGlobal { name });
        }
      }

      Ok(())
    }

    fn emit_get_named_expr(&mut self, expr: &'src ast::GetNamed<'src>) -> Result<()> {
      let name = self.const_(expr.name.deref().clone());
      self.emit_expr(&expr.target)?;
      self.emit_op(LoadNamed { name });

      Ok(())
    }

    fn emit_set_named_expr(&mut self, expr: &'src ast::SetNamed<'src>) -> Result<()> {
      let obj = self.reg();
      let name = self.const_(expr.target.name.deref().clone());
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
      self.emit_op(LoadKeyed { key: key.index() });

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
        self.emit_expr(&value)?;
        self.emit_op(StoreReg { reg: reg.index() });
      }

      for (key, value) in expr.args.kw.iter() {
        let key = self.const_(key.as_ref());
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
