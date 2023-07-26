use core::cell::{Cell, UnsafeCell};
use core::fmt::{Debug, Display};
use core::mem::replace;

use crate::ds::vec::{GcVec, GcVecN};
use crate::ds::{HasAlloc, HasNoAlloc};
use crate::error::{AllocError, Error, Result};
use crate::gc::{Alloc, Any, Gc, Object, Ref};
use crate::obj::func::Function;
use crate::obj::module::ModuleRegistry;
use crate::op::{Op, Reg};
use crate::val::{nil, Value};

macro_rules! error {
  ($e:expr, $ip:ident, $frame:ident, $registry:ident) => {{
    let ops = $frame.function.ops();
    let span = $frame.function.dbg().loc()[get_pc($ip, ops)];
    let module = $registry.by_id($frame.function.module_id()).unwrap();
    Error::runtime(
      module.name().as_str(),
      $frame.function.dbg().src(),
      $e.to_string(),
      span,
    )
  }};
}

macro_rules! errorf {
  ($ip:ident, $frame:ident, $registry:ident) => {
    #[inline(never)]
    |e| error!(e, $ip, $frame, $registry)
  };
}

#[derive(Debug)]
pub struct Thread {
  stack: Ref<Stack>,
  pc: usize,
}

impl Thread {
  pub fn new(gc: &Gc) -> Result<Ref<Self>, AllocError> {
    let stack = Stack::new(gc)?;
    gc.try_alloc(Thread { stack, pc: 0 })
  }

  pub fn run(
    &self,
    gc: &Gc,
    registry: Ref<ModuleRegistry>,
    function: Ref<Function>,
  ) -> Result<Value> {
    match self.try_run(gc, registry, function) {
      Ok(v) => Ok(v),
      Err(e) => {
        // unwind stack
        for _ in self.stack.cstack(gc).drain(..).rev() {
          // TODO: backtrace
        }

        Err(e)
      }
    }
  }

  fn try_run(
    &self,
    gc: &Gc,
    registry: Ref<ModuleRegistry>,
    function: Ref<Function>,
  ) -> Result<Value> {
    let mut pc = self.pc;
    let mut current_frame = CallFrame::new(function, 0);

    {
      let frame_size = function.frame_size() as usize;
      self.stack.set_stack_top(frame_size - 1);
      let vstack = self.stack.vstack(gc);
      vstack.try_reserve(frame_size).map_err(|e| {
        // TODO: actual error here
        Error::simple(e.to_string())
      })?;
      vstack.extend((0..frame_size).map(|_| Value::new(nil)));
    }

    'frame: loop {
      let ops = current_frame.function.ops();
      let pool = current_frame.function.pool();
      let captures = current_frame.function.captures();
      let vars = registry
        .by_id(current_frame.function.module_id())
        .expect("registry does not contain running function's module id")
        .vars();

      let r = &mut self.stack.vstack(gc)[current_frame.base..]
        [..current_frame.function.frame_size() as usize];

      let mut ip = unsafe { ops.as_ptr().add(pc) };
      'dispatch: loop {
        // println!("{:?}", unsafe { *ip });
        match unsafe { *ip } {
          Op::Nop => ip = unsafe { ip.add(1) },
          Op::Mov { src, dst } => {
            r[dst.wide()] = r[src.wide()];
            ip = unsafe { ip.add(1) };
            continue 'dispatch;
          }
          Op::LoadConst { dst, idx } => {
            r[dst.wide()] = pool.get(idx.wide()).unwrap().value();
            ip = unsafe { ip.add(1) };
            continue 'dispatch;
          }
          Op::LoadCapture { dst, idx } => {
            r[dst.wide()] = captures.get(idx.wide()).unwrap();
            ip = unsafe { ip.add(1) };
            continue 'dispatch;
          }
          Op::StoreCapture { src, idx } => {
            assert!(captures.set(idx.wide(), r[src.wide()]));
            ip = unsafe { ip.add(1) };
            continue 'dispatch;
          }
          Op::LoadMvar { dst, idx } => {
            r[dst.wide()] = vars.get_index(idx.wide()).unwrap();
            ip = unsafe { ip.add(1) };
            continue 'dispatch;
          }
          Op::StoreMvar { src, idx } => {
            assert!(vars.set_index(idx.wide(), r[src.wide()]));
            ip = unsafe { ip.add(1) };
            continue 'dispatch;
          }
          Op::LoadGlobal { dst, key } => {
            todo!();
          }
          Op::StoreGlobal { src, key } => {
            todo!();
          }
          Op::LoadField { obj, key, dst } => {
            todo!();
          }
          Op::LoadFieldR { obj, key, dst } => {
            todo!();
          }
          Op::LoadFieldOpt { obj, key, dst } => {
            todo!();
          }
          Op::LoadFieldROpt { obj, key, dst } => {
            todo!();
          }
          Op::LoadFieldInt { obj, key, dst } => {
            todo!();
          }
          Op::LoadFieldIntR { obj, key, dst } => {
            todo!();
          }
          Op::LoadFieldIntOpt { obj, key, dst } => {
            todo!();
          }
          Op::LoadFieldIntROpt { obj, key, dst } => {
            todo!();
          }
          Op::StoreField { obj, key, src } => {
            todo!();
          }
          Op::StoreFieldR { obj, key, src } => {
            todo!();
          }
          Op::StoreFieldInt { obj, key, src } => {
            todo!();
          }
          Op::StoreFieldIntR { obj, key, src } => {
            todo!();
          }
          Op::LoadIndex { obj, key, dst } => {
            todo!();
          }
          Op::LoadIndexOpt { obj, key, dst } => {
            todo!();
          }
          Op::StoreIndex { obj, key, src } => {
            todo!();
          }
          Op::LoadSuper { dst } => {
            todo!();
          }
          Op::LoadNil { dst } => {
            r[dst.wide()] = Value::new(nil);
            ip = unsafe { ip.add(1) };
            continue 'dispatch;
          }
          Op::LoadTrue { dst } => {
            r[dst.wide()] = Value::new(true);
            ip = unsafe { ip.add(1) };
            continue 'dispatch;
          }
          Op::LoadFalse { dst } => {
            r[dst.wide()] = Value::new(false);
            ip = unsafe { ip.add(1) };
            continue 'dispatch;
          }
          Op::LoadSmi { dst, value } => {
            r[dst.wide()] = Value::new(value.wide());
            ip = unsafe { ip.add(1) };
            continue 'dispatch;
          }
          Op::MakeFn { dst, desc } => {
            let proto = pool[desc.wide()].function();
            let f =
              Function::new(gc, proto, captures).map_err(errorf!(ip, current_frame, registry))?;
            r[dst.wide()] = Value::new(f);
            ip = unsafe { ip.add(1) };
            continue 'dispatch;
          }
          Op::MakeClass { dst, desc } => {
            todo!();
          }
          Op::MakeClassDerived { dst, desc } => {
            todo!();
          }
          Op::MakeList { dst, desc } => {
            todo!();
          }
          Op::MakeListEmpty { dst } => {
            todo!();
          }
          Op::MakeMap { dst, desc } => {
            todo!();
          }
          Op::MakeMapEmpty { dst } => {
            todo!();
          }
          Op::MakeTuple { dst, desc } => {
            todo!();
          }
          Op::MakeTupleEmpty { dst } => {
            todo!();
          }
          Op::Jump { offset } => {
            ip = unsafe { ip.add(offset.wide()) };
            continue 'dispatch;
          }
          Op::JumpConst { idx } => {
            let offset = pool[idx.wide()].offset();
            ip = unsafe { ip.add(offset.wide()) };
            continue 'dispatch;
          }
          Op::JumpLoop { offset } => {
            ip = unsafe { ip.sub(offset.wide()) };
            continue 'dispatch;
          }
          Op::JumpLoopConst { idx } => {
            let offset = pool[idx.wide()].offset();
            ip = unsafe { ip.add(offset.wide()) };
            continue 'dispatch;
          }
          Op::JumpIfFalse { val, offset } => {
            if r[val.wide()].is_truthy() {
              ip = unsafe { ip.add(1) };
            } else {
              ip = unsafe { ip.add(offset.wide()) };
            }
            continue 'dispatch;
          }
          Op::JumpIfFalseConst { val, idx } => {
            if r[val.wide()].is_truthy() {
              ip = unsafe { ip.add(1) };
            } else {
              let offset = pool[idx.wide()].offset();
              ip = unsafe { ip.add(offset.wide()) };
            }
            continue 'dispatch;
          }
          Op::JumpIfTrue { val, offset } => {
            if !r[val.wide()].is_truthy() {
              ip = unsafe { ip.add(1) };
            } else {
              ip = unsafe { ip.add(offset.wide()) };
            }
            continue 'dispatch;
          }
          Op::JumpIfTrueConst { val, idx } => {
            if !r[val.wide()].is_truthy() {
              ip = unsafe { ip.add(1) };
            } else {
              let offset = pool[idx.wide()].offset();
              ip = unsafe { ip.add(offset.wide()) };
            }
            continue 'dispatch;
          }
          Op::Add { dst, lhs, rhs } => {
            r[dst.wide()] = add(r[lhs.wide()], r[rhs.wide()]).map_err(
              #[inline(never)]
              |e| error!(e, ip, current_frame, registry),
            )?;
            ip = unsafe { ip.add(1) };
            continue 'dispatch;
          }
          Op::Sub { dst, lhs, rhs } => {
            r[dst.wide()] = sub(r[lhs.wide()], r[rhs.wide()]).map_err(
              #[inline(never)]
              |e| error!(e, ip, current_frame, registry),
            )?;
            ip = unsafe { ip.add(1) };
            continue 'dispatch;
          }
          Op::Mul { dst, lhs, rhs } => {
            r[dst.wide()] = mul(r[lhs.wide()], r[rhs.wide()]).map_err(
              #[inline(never)]
              |e| error!(e, ip, current_frame, registry),
            )?;
            ip = unsafe { ip.add(1) };
            continue 'dispatch;
          }
          Op::Div { dst, lhs, rhs } => {
            r[dst.wide()] = div(r[lhs.wide()], r[rhs.wide()]).map_err(
              #[inline(never)]
              |e| error!(e, ip, current_frame, registry),
            )?;
            ip = unsafe { ip.add(1) };
            continue 'dispatch;
          }
          Op::Rem { dst, lhs, rhs } => {
            r[dst.wide()] = rem(r[lhs.wide()], r[rhs.wide()]).map_err(
              #[inline(never)]
              |e| error!(e, ip, current_frame, registry),
            )?;
            ip = unsafe { ip.add(1) };
            continue 'dispatch;
          }
          Op::Pow { dst, lhs, rhs } => {
            r[dst.wide()] = pow(r[lhs.wide()], r[rhs.wide()]).map_err(
              #[inline(never)]
              |e| error!(e, ip, current_frame, registry),
            )?;
            ip = unsafe { ip.add(1) };
            continue 'dispatch;
          }
          Op::Inv { val } => {
            r[val.wide()] = inv(r[val.wide()]).map_err(
              #[inline(never)]
              |e| error!(e, ip, current_frame, registry),
            )?;
            ip = unsafe { ip.add(1) };
            continue 'dispatch;
          }
          Op::Not { val } => {
            r[val.wide()] = not(r[val.wide()]).map_err(
              #[inline(never)]
              |e| error!(e, ip, current_frame, registry),
            )?;
            ip = unsafe { ip.add(1) };
            continue 'dispatch;
          }
          Op::CmpEq { dst, lhs, rhs } => {
            r[dst.wide()] = cmp_eq(r[lhs.wide()], r[rhs.wide()]).map_err(
              #[inline(never)]
              |e| error!(e, ip, current_frame, registry),
            )?;
            ip = unsafe { ip.add(1) };
          }
          Op::CmpNe { dst, lhs, rhs } => {
            r[dst.wide()] = cmp_ne(r[lhs.wide()], r[rhs.wide()]).map_err(
              #[inline(never)]
              |e| error!(e, ip, current_frame, registry),
            )?;
            ip = unsafe { ip.add(1) };
          }
          Op::CmpGt { dst, lhs, rhs } => {
            r[dst.wide()] = cmp_gt(r[lhs.wide()], r[rhs.wide()]).map_err(
              #[inline(never)]
              |e| error!(e, ip, current_frame, registry),
            )?;
            ip = unsafe { ip.add(1) };
          }
          Op::CmpGe { dst, lhs, rhs } => {
            r[dst.wide()] = cmp_ge(r[lhs.wide()], r[rhs.wide()]).map_err(
              #[inline(never)]
              |e| error!(e, ip, current_frame, registry),
            )?;
            ip = unsafe { ip.add(1) };
          }
          Op::CmpLt { dst, lhs, rhs } => {
            r[dst.wide()] = cmp_lt(r[lhs.wide()], r[rhs.wide()]).map_err(
              #[inline(never)]
              |e| error!(e, ip, current_frame, registry),
            )?;
            ip = unsafe { ip.add(1) };
          }
          Op::CmpLe { dst, lhs, rhs } => {
            r[dst.wide()] = cmp_le(r[lhs.wide()], r[rhs.wide()]).map_err(
              #[inline(never)]
              |e| error!(e, ip, current_frame, registry),
            )?;
            ip = unsafe { ip.add(1) };
          }
          Op::CmpType { dst, lhs, rhs } => {
            r[dst.wide()] = cmp_type(r[lhs.wide()], r[rhs.wide()]).map_err(
              #[inline(never)]
              |e| error!(e, ip, current_frame, registry),
            )?;
            ip = unsafe { ip.add(1) };
          }
          Op::Contains { dst, lhs, rhs } => {
            r[dst.wide()] = contains(r[lhs.wide()], r[rhs.wide()]).map_err(
              #[inline(never)]
              |e| error!(e, ip, current_frame, registry),
            )?;
            ip = unsafe { ip.add(1) };
          }
          Op::IsNil { dst, val } => {
            r[dst.wide()] = Value::new(r[val.wide()].is::<nil>());
            ip = unsafe { ip.add(1) };
          }
          Op::Call { func, count } => {
            let base = current_frame.base + func.wide();

            let func = r[func.wide()];
            let func = func
              .cast::<Ref<Function>>()
              .ok_or_else(|| error!(format!("cannot call {func}"), ip, current_frame, registry))?;

            if func.arity() != count.wide() {
              return Err(error!(
                format!("expected {} arguments, got {}", func.arity(), count.wide()),
                ip, current_frame, registry
              ));
            }

            let return_addr = get_pc(ip, ops) + 1;
            let previous_frame = replace(
              &mut current_frame,
              CallFrame::with_return_addr(func, base, return_addr),
            );

            {
              if let Err(e) = extend_stack(gc, &func, &self.stack, base, count.wide()) {
                return Err(error!(e, ip, previous_frame, registry));
              }
              self.stack.cstack(gc).push(previous_frame);
              update_stack_top(&self.stack, &current_frame);
              pc = 0;
            }

            continue 'frame;
          }
          Op::Call0 { func } => {
            let base = current_frame.base + func.wide();

            let func = r[func.wide()];
            let func = func
              .cast::<Ref<Function>>()
              .ok_or_else(|| error!(format!("cannot call {func}"), ip, current_frame, registry))?;

            if func.arity() != 0 {
              return Err(error!(
                format!("expected 0 arguments, got {}", func.arity()),
                ip, current_frame, registry
              ));
            }

            let return_addr = get_pc(ip, ops) + 1;
            let previous_frame = replace(
              &mut current_frame,
              CallFrame::with_return_addr(func, base, return_addr),
            );

            {
              if let Err(e) = extend_stack(gc, &func, &self.stack, base, 0) {
                return Err(error!(e, ip, previous_frame, registry));
              }
              self.stack.cstack(gc).push(previous_frame);
              update_stack_top(&self.stack, &current_frame);
              pc = 0;
            }

            continue 'frame;
          }
          Op::Import { dst, path } => {
            todo!();
          }
          Op::FinalizeModule => {
            // TODO: actually finalize
            ip = unsafe { ip.add(1) };
          }
          Op::Ret { val } => {
            r[0] = r[val.wide()];
            if let Some(previous_frame) = self.stack.cstack(gc).pop() {
              pc = current_frame.return_addr.unwrap_or(0);
              current_frame = previous_frame;
              update_stack_top(&self.stack, &current_frame);
              continue 'frame;
            } else {
              self.stack.set_stack_top(0);
              return Ok(r[0]);
            }
          }
          Op::Yld { val } => {
            todo!();
          }
        }
      }
    }
  }
}

#[inline]
fn extend_stack(
  gc: &Gc,
  func: &Function,
  stack: &Stack,
  base: usize,
  args: usize,
) -> Result<(), AllocError> {
  // check if vstack has enough space above `base` to fit `frame_size`
  //   if it doesn't, extend it

  let frame_size = func.frame_size() as usize;
  let vstack = stack.vstack(gc);
  let remaining_length = vstack.len() - base;
  let extra_length = frame_size - 1 - args;
  if remaining_length < extra_length {
    let additional = extra_length - remaining_length;
    vstack.try_reserve(additional).map_err(|_| AllocError)?;
    vstack.extend((0..additional).map(|_| Value::new(nil)));
  }
  Ok(())
}

#[inline]
fn update_stack_top(stack: &Stack, frame: &CallFrame) {
  let base = frame.base;
  let frame_size = frame.function.frame_size() as usize;
  stack.set_stack_top(base + frame_size - 1);
}

macro_rules! match_type {
  (
    $(
      ($($binding:ident : $ty:ty),*) => $body:expr,
    )*
    else => $default:expr $(,)?
  ) => {{
    #![allow(unused_variables, unreachable_code)]

    $(
      if let ($(Some($binding),)*) = ($($binding.cast::<$ty>(),)*) {
        return $body
      }
    )*
    $default
  }};
}

struct TypeMismatch;

impl Display for TypeMismatch {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.write_str("type mismatch")
  }
}

fn add(lhs: Value, rhs: Value) -> Result<Value, TypeMismatch> {
  match_type! {
    (lhs: f64, rhs: f64) => Ok(Value::new(lhs + rhs)),
    (lhs: i32, rhs: i32) => Ok(Value::new(lhs + rhs)),
    (lhs: f64, rhs: i32) => Ok(Value::new(lhs + rhs as f64)),
    (lhs: i32, rhs: f64) => Ok(Value::new(lhs as f64 + rhs)),
    (lhs: Any, rhs: Any) => todo!(),
    else => Err(TypeMismatch)
  }
}

fn sub(lhs: Value, rhs: Value) -> Result<Value, TypeMismatch> {
  match_type! {
    (lhs: f64, rhs: f64) => Ok(Value::new(lhs - rhs)),
    (lhs: i32, rhs: i32) => Ok(Value::new(lhs - rhs)),
    (lhs: f64, rhs: i32) => Ok(Value::new(lhs - rhs as f64)),
    (lhs: i32, rhs: f64) => Ok(Value::new(lhs as f64 - rhs)),
    (lhs: Any, rhs: Any) => todo!(),
    else => Err(TypeMismatch)
  }
}

fn mul(lhs: Value, rhs: Value) -> Result<Value, TypeMismatch> {
  match_type! {
    (lhs: f64, rhs: f64) => Ok(Value::new(lhs * rhs)),
    (lhs: i32, rhs: i32) => Ok(Value::new(lhs * rhs)),
    (lhs: f64, rhs: i32) => Ok(Value::new(lhs * rhs as f64)),
    (lhs: i32, rhs: f64) => Ok(Value::new(lhs as f64 * rhs)),
    (lhs: Any, rhs: Any) => todo!(),
    else => Err(TypeMismatch)
  }
}

enum DivisionError {
  TypeMismatch,
  DivideByZero,
}

impl Display for DivisionError {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      DivisionError::TypeMismatch => f.write_str("type mismatch"),
      DivisionError::DivideByZero => f.write_str("divide by zero"),
    }
  }
}

fn div(lhs: Value, rhs: Value) -> Result<Value, DivisionError> {
  match_type! {
    (lhs: f64, rhs: f64) => {
      if rhs == 0.0 {
        return Err(DivisionError::DivideByZero);
      }
      Ok(Value::new(lhs / rhs))
    },
    (lhs: i32, rhs: i32) => {
      if rhs == 0 {
        return Err(DivisionError::DivideByZero);
      }
      Ok(Value::new(lhs / rhs))
    },
    (lhs: f64, rhs: i32) => {
      if rhs == 0 {
        return Err(DivisionError::DivideByZero);
      }
      Ok(Value::new(lhs / rhs as f64))
    },
    (lhs: i32, rhs: f64) => {
      if rhs == 0.0 {
        return Err(DivisionError::DivideByZero);
      }
      Ok(Value::new(lhs as f64 / rhs))
    },
    (lhs: Any, rhs: Any) => todo!(),
    else => Err(DivisionError::TypeMismatch)
  }
}

fn rem(lhs: Value, rhs: Value) -> Result<Value, DivisionError> {
  match_type! {
    (lhs: f64, rhs: f64) => {
      if rhs == 0.0 {
        return Err(DivisionError::DivideByZero);
      }
      Ok(Value::new(lhs % rhs))
    },
    (lhs: i32, rhs: i32) => {
      if rhs == 0 {
        return Err(DivisionError::DivideByZero);
      }
      Ok(Value::new(lhs % rhs))
    },
    (lhs: f64, rhs: i32) => {
      if rhs == 0 {
        return Err(DivisionError::DivideByZero);
      }
      Ok(Value::new(lhs % rhs as f64))
    },
    (lhs: i32, rhs: f64) => {
      if rhs == 0.0 {
        return Err(DivisionError::DivideByZero);
      }
      Ok(Value::new(lhs as f64 % rhs))
    },
    (lhs: Any, rhs: Any) => todo!(),
    else => Err(DivisionError::TypeMismatch)
  }
}

fn pow(lhs: Value, rhs: Value) -> Result<Value, TypeMismatch> {
  match_type! {
    (lhs: f64, rhs: f64) => Ok(Value::new(lhs.powf(rhs))),
    (lhs: i32, rhs: i32) => {
      if rhs < 0 {
        Ok(Value::new((lhs as f64).powi(rhs)))
      } else {
        Ok(Value::new(lhs.pow(rhs as u32)))
      }
    },
    (lhs: f64, rhs: i32) => Ok(Value::new(lhs.powi(rhs))),
    (lhs: i32, rhs: f64) => Ok(Value::new((lhs as f64).powf(rhs))),
    (lhs: Any, rhs: Any) => todo!(),
    else => Err(TypeMismatch)
  }
}

fn inv(val: Value) -> Result<Value, TypeMismatch> {
  match_type! {
    (val: f64) => Ok(Value::new(-val)),
    (val: i32) => Ok(Value::new(-val)),
    (val: Any) => todo!(),
    else => Err(TypeMismatch),
  }
}

fn not(val: Value) -> Result<Value, TypeMismatch> {
  Ok(Value::new(!val.is_truthy()))
}

fn cmp_eq(lhs: Value, rhs: Value) -> Result<Value, TypeMismatch> {
  match_type! {
    (lhs: f64, rhs: f64) => Ok(Value::new(lhs == rhs)),
    (lhs: i32, rhs: i32) => Ok(Value::new(lhs == rhs)),
    (lhs: f64, rhs: i32) => Ok(Value::new(lhs == rhs as f64)),
    (lhs: i32, rhs: f64) => Ok(Value::new(lhs as f64 == rhs)),
    (lhs: Any, rhs: Any) => todo!(),
    else => Ok(Value::new(false)),
  }
}

fn cmp_ne(lhs: Value, rhs: Value) -> Result<Value, TypeMismatch> {
  match_type! {
    (lhs: f64, rhs: f64) => Ok(Value::new(lhs != rhs)),
    (lhs: i32, rhs: i32) => Ok(Value::new(lhs != rhs)),
    (lhs: f64, rhs: i32) => Ok(Value::new(lhs != rhs as f64)),
    (lhs: i32, rhs: f64) => Ok(Value::new(lhs as f64 != rhs)),
    (lhs: Any, rhs: Any) => todo!(),
    else => Ok(Value::new(false)),
  }
}

fn cmp_gt(lhs: Value, rhs: Value) -> Result<Value, TypeMismatch> {
  match_type! {
    (lhs: f64, rhs: f64) => Ok(Value::new(lhs > rhs)),
    (lhs: i32, rhs: i32) => Ok(Value::new(lhs > rhs)),
    (lhs: f64, rhs: i32) => Ok(Value::new(lhs > rhs as f64)),
    (lhs: i32, rhs: f64) => Ok(Value::new(lhs as f64 > rhs)),
    (lhs: Any, rhs: Any) => todo!(),
    else => Ok(Value::new(false)),
  }
}

fn cmp_ge(lhs: Value, rhs: Value) -> Result<Value, TypeMismatch> {
  match_type! {
    (lhs: f64, rhs: f64) => Ok(Value::new(lhs >= rhs)),
    (lhs: i32, rhs: i32) => Ok(Value::new(lhs >= rhs)),
    (lhs: f64, rhs: i32) => Ok(Value::new(lhs >= rhs as f64)),
    (lhs: i32, rhs: f64) => Ok(Value::new(lhs as f64 >= rhs)),
    (lhs: Any, rhs: Any) => todo!(),
    else => Ok(Value::new(false)),
  }
}

fn cmp_lt(lhs: Value, rhs: Value) -> Result<Value, TypeMismatch> {
  match_type! {
    (lhs: f64, rhs: f64) => Ok(Value::new(lhs < rhs)),
    (lhs: i32, rhs: i32) => Ok(Value::new(lhs < rhs)),
    (lhs: f64, rhs: i32) => Ok(Value::new(lhs < rhs as f64)),
    (lhs: i32, rhs: f64) => Ok(Value::new((lhs as f64) < rhs)),
    (lhs: Any, rhs: Any) => todo!(),
    else => Ok(Value::new(false)),
  }
}

fn cmp_le(lhs: Value, rhs: Value) -> Result<Value, TypeMismatch> {
  match_type! {
    (lhs: f64, rhs: f64) => Ok(Value::new(lhs <= rhs)),
    (lhs: i32, rhs: i32) => Ok(Value::new(lhs <= rhs)),
    (lhs: f64, rhs: i32) => Ok(Value::new(lhs <= rhs as f64)),
    (lhs: i32, rhs: f64) => Ok(Value::new(lhs as f64 <= rhs)),
    (lhs: Any, rhs: Any) => todo!(),
    else => Ok(Value::new(false)),
  }
}

fn cmp_type(lhs: Value, rhs: Value) -> Result<Value, TypeMismatch> {
  todo!()
}

fn contains(lhs: Value, rhs: Value) -> Result<Value, TypeMismatch> {
  todo!()
}

fn get_pc(ip: *const Op, ops: *const [Op]) -> usize {
  ((ip as usize) - (ops as *const Op as usize)) / 4
}

#[derive(Debug)]
pub struct Stack {
  cstack: UnsafeCell<GcVecN<CallFrame>>,
  vstack: UnsafeCell<GcVecN<Value>>,
  /// Any value in `vstack` past `stack_top` is unreachable
  stack_top: Cell<usize>,
}

impl Stack {
  fn new(gc: &Gc) -> Result<Ref<Self>, AllocError> {
    let cstack = GcVec::with_capacity_in(64, Alloc::new(gc));
    let mut vstack = GcVec::with_capacity_in(64, Alloc::new(gc));
    vstack.extend((0..64).map(|_| Value::new(nil)));

    gc.try_alloc(Stack {
      cstack: UnsafeCell::new(cstack.to_no_alloc()),
      vstack: UnsafeCell::new(vstack.to_no_alloc()),
      stack_top: Cell::new(0),
    })
  }

  #[allow(clippy::mut_from_ref)]
  fn cstack<'gc>(&self, gc: &'gc Gc) -> &mut GcVec<'gc, CallFrame> {
    let cstack = unsafe { self.cstack.get().as_mut().unwrap_unchecked() };
    cstack.as_alloc_mut(gc)
  }

  #[allow(clippy::mut_from_ref)]
  fn vstack<'gc>(&self, gc: &'gc Gc) -> &mut GcVec<'gc, Value> {
    let vstack = unsafe { self.vstack.get().as_mut().unwrap_unchecked() };
    vstack.as_alloc_mut(gc)
  }

  fn set_stack_top(&self, stack_top: usize) {
    self.stack_top.set(stack_top)
  }
}

pub struct CallFrame {
  function: Ref<Function>,
  return_addr: Option<usize>,
  base: usize,
}

impl CallFrame {
  fn new(function: Ref<Function>, base: usize) -> Self {
    Self {
      function,
      return_addr: None,
      base,
    }
  }

  fn with_return_addr(function: Ref<Function>, base: usize, return_addr: usize) -> Self {
    Self {
      function,
      return_addr: Some(return_addr),
      base,
    }
  }
}

impl Debug for CallFrame {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    f.debug_struct("CallFrame")
      .field("function", &self.function)
      .field("base", &self.base)
      .field("return_addr", &self.return_addr)
      .finish()
  }
}

impl Display for Thread {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "stack:\n{}", self.stack)
  }
}

impl Object for Thread {}

impl Display for Stack {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    let cstack = unsafe { self.cstack.get().as_ref().unwrap() };
    let vstack = unsafe { self.vstack.get().as_ref().unwrap() };
    for frame in cstack.iter() {
      let base = frame.base;
      let frame_size = frame.function.frame_size() as usize;
      writeln!(
        f,
        "[{} {}..{}]",
        frame.function.name(),
        base,
        base + frame_size
      )?;
      for (i, v) in vstack[base..][..frame_size].iter().enumerate() {
        let i = Reg(i);
        writeln!(f, "  {i}: {v}")?;
      }
    }
    Ok(())
  }
}

impl Object for Stack {
  const NEEDS_DROP: bool = false;
}
