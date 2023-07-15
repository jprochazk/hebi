#![allow(clippy::needless_lifetimes)]

mod builder;

use core::hash::BuildHasherDefault;

use alloc::format;
use bumpalo::collections::Vec;
use bumpalo::vec;
use rustc_hash::FxHasher;

use crate::ast::Expr;
use crate::ast::Func;
use crate::ast::Let;
use crate::ast::Loop;
use crate::ast::Module;
use crate::ast::Return;
use crate::ast::Stmt;
use crate::gc::{Gc, Ref};
use crate::obj::module::ModuleDescriptor;
use crate::obj::string::Str;
use crate::op::Offset;
use crate::Arena;

use self::builder::BytecodeBuilder;

use super::Mvar;
use super::Op;
use super::Reg;
use crate::op;
use crate::op::asm::*;

type HashSet<T, A> = hashbrown::HashSet<T, BuildHasherDefault<FxHasher>, A>;
type HashMap<K, V, A> = hashbrown::HashMap<K, V, BuildHasherDefault<FxHasher>, A>;

pub type Result<T, E = alloc::string::String> = core::result::Result<T, E>;

pub fn module<'arena, 'gc, 'src>(
  arena: &'arena Arena,
  gc: &'gc Gc,
  name: &'src str,
  ast: Module<'arena, 'src>,
) -> Result<Ref<ModuleDescriptor>> {
  let mut state = ModuleState {
    arena,
    gc,
    name,
    ast,

    module_vars: HashMap::with_hasher_in(BuildHasherDefault::default(), arena),
  };

  let mut state = FunctionState {
    module: &mut state,
    parent: None,

    arena,
    gc,
    name,

    builder: BytecodeBuilder::new_in(arena),
    registers: 0,
    scope: Scope(0),
    locals: HashMap::with_hasher_in(BuildHasherDefault::default(), arena),
  };

  top_level(&mut state)
}

struct ModuleState<'arena, 'gc, 'src> {
  arena: &'arena Arena,
  gc: &'gc Gc,
  name: &'src str,
  ast: Module<'arena, 'src>,

  module_vars: HashMap<Ref<Str>, Mvar<usize>, &'arena Arena>,
}

struct FunctionState<'arena, 'gc, 'src, 'state> {
  module: &'state mut ModuleState<'arena, 'gc, 'src>,
  parent: Option<&'state mut FunctionState<'arena, 'gc, 'src, 'state>>,

  arena: &'arena Arena,
  gc: &'gc Gc,
  name: &'src str,

  builder: BytecodeBuilder<'arena>,
  registers: usize,
  scope: Scope,
  locals: HashMap<(Scope, &'src str), Reg<u8>, &'arena Arena>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Scope(usize);

impl<'arena, 'gc, 'src, 'state> FunctionState<'arena, 'gc, 'src, 'state> {
  #[inline]
  fn scope<F, T>(&mut self, f: F) -> Result<T>
  where
    F: FnOnce(&mut FunctionState<'arena, 'gc, 'src, 'state>) -> Result<T>,
  {
    self.scope.0 += 1;
    let result = f(self);
    self.scope.0 -= 1;
    result
  }

  #[doc(hidden)]
  #[inline]
  fn _reg(&mut self) -> Reg<u8> {
    let reg = self.registers as u8;
    self.registers += 1;
    Reg(reg)
  }

  #[inline]
  fn reg(&mut self) -> Result<Reg<u8>> {
    if self.registers > u8::MAX as usize {
      return Err(format!("function `{}` uses too many registers", self.name));
    }
    Ok(self._reg())
  }

  #[inline]
  fn free(&mut self, r: Reg<u8>) {
    self.registers = r.wide();
  }

  #[inline]
  fn emit(&mut self, op: Op) {
    self.builder.emit(op);
  }

  #[inline]
  fn is_top_level(&self) -> bool {
    self.parent.is_none()
  }
}

fn top_level(state: &mut FunctionState) -> Result<Ref<ModuleDescriptor>> {
  state.scope(|state| {
    for node in state.module.ast {
      stmt(state, node)?;
    }
    Ok(())
  })?;

  todo!()
}

fn stmt(state: &mut FunctionState, node: &Stmt) -> Result<()> {
  match node.kind {
    crate::ast::StmtKind::Let(node) => let_(state, node),
    crate::ast::StmtKind::Loop(node) => loop_(state, node),
    crate::ast::StmtKind::Break => break_(state),
    crate::ast::StmtKind::Continue => continue_(state),
    crate::ast::StmtKind::Return(node) => return_(state, node),
    crate::ast::StmtKind::Func(node) => func(state, node),
    crate::ast::StmtKind::Expr(node) => {
      let dst = state.reg()?;
      expr(state, dst, node)?;
      state.free(dst);
      Ok(())
    }
  }
}

fn let_(state: &mut FunctionState, node: &Let) -> Result<()> {
  let dst = state.reg()?;

  if let Some(value) = &node.value {
    if let Some(out) = expr(state, dst, value)? {
      // `expr` was written to `out`
      state.emit(mov(out, dst));
    } else {
      // `expr` was written out to `dst`
    }
  } else {
    state.emit(load_none(dst));
  }

  if state.is_top_level() {
    // TODO: emit module var
  } /* else {
      // TODO: emit local
    } */

  todo!()
}

fn loop_(state: &mut FunctionState, node: &Loop) -> Result<()> {
  todo!()
}

fn break_(state: &mut FunctionState) -> Result<()> {
  todo!()
}

fn continue_(state: &mut FunctionState) -> Result<()> {
  todo!()
}

fn return_(state: &mut FunctionState, node: &Return) -> Result<()> {
  todo!()
}

fn func(state: &mut FunctionState, node: &Func) -> Result<()> {
  todo!()
}

fn expr(state: &mut FunctionState, dst: Reg<u8>, node: &Expr) -> Result<Option<Reg<u8>>> {
  todo!()
}

/*

register allocation:

follows a stack discipline, registers are allocated as needed

- temporary registers used for intermediate results in expressions
  are freed at the end of their local scope, which is immediately
  before exiting the function which emits the expression

- variables are stored in `state.locals`, and freed upon exiting the current scope

  let v = 10 + 10
  ^ enter let_

  let v = 10 + 10
      ^ allocate register `r0`

  let v = 10 + 10
          ^ enter expr(r0)

  let v = 10 + 10
          ^ lhs emitted first, enter literal(r0)

  let v = 10 + 10
          ^ literal is directly written to `r0`
            no register is returned

  let v = 10 + 10
               ^ rhs is emitted next
                 allocate fresh register (r1)
                 enter literal(r1)

  let v = 10 + 10
               ^ literal is written to `r1`
                 no register is returned

  let v = 10 + 10
          ^ lhs and rhs are emitted,
            emit `add r1, r1, r0`
            return no register
            # Q: Is there ever a scenario where we _wouldn't_ use `r1` as `dst`?
              A: no, but even though `dst` will always be `r1`,
                 the `lhs` or `rhs` may be variables, which would mean
                 that `dst` is not always equal to `lhs`

  # exit all the way to let_

  let v = 10 + 10
      ^ received no register from `value`
        do not emit anything
        if global scope: define `v` as mvar (set_mvar(r0))
        else:            define `v` as local (add `r0` to locals)

  let u = v + 10
  ^ enter let_

  let u = v + 10
      ^ allocate register `r1`

  let u = v + 10
          ^ enter expr(r1)

  let u = v + 10
          ^ lhs first, enter get_var(r1)

  let u = v + 10
          ^ do not emit anything
            return register `r0`

  let u = v + 10
              ^ alloc fresh register (r2)
                enter expr(r2)

  let u = v + 10
              ^ enter literal(r2)

  let u = v + 10
              ^ literal is written to r2

  let u = v + 10
          ^ lhs=r0, rhs=r2
            emit(add r1, r0, r2)
            return no register

  let u = v + 10
      ^ got no register
        do not emit anything
        if global scope: define `u` as mvar (set_mvar(r1))
        else:            define `u` as local (add `r1` to locals)



  # Q: Can we use only 2 registers for the output of:
  10 + 10 + 10 + 10
  ^ expr(r0)
  ^ literal(r0)
  10 + 10 + 10 + 10
       ^ expr(r1)
       ^ literal(r1)
  10 + 10 + 10 + 10
            ^ expr(r2)
            ^ literal(r2)
  10 + 10 + 10 + 10
                 ^ expr(r3)
                 ^ literal(r3)
  10 + 10 + 10 + 10
               ^ add r2, r2, r3
  10 + 10 + 10 + 10
          ^ add r1, r1, r2
  10 + 10 + 10 + 10
     ^ add r0, r0, r1

  # A: No, we have to use extra registers where possible


*/
