#[macro_use]
mod macros;

use std::error::Error as StdError;
use std::fmt::Display;
use std::ptr::NonNull;

use crate::bytecode::opcode as op;
use crate::bytecode::opcode::Opcode;
use crate::bytecode::operands::Width;

pub fn dispatch<T: Handler>(
  handler: &mut T,
  bytecode: NonNull<[u8]>,
  pc: usize,
) -> Result<ControlFlow, Error<T::Error>> {
  let mut bytecode = bytecode;
  let mut pc = pc;

  'load_frame: loop {
    let ip = bytecode.as_ptr() as *mut u8;
    if pc >= bytecode.len() {
      return Err(Error::UnexpectedEnd);
    }
    let end = unsafe { ip.add(bytecode.len()) };
    let mut ip = unsafe { ip.add(pc) };
    let mut width = Width::Normal;

    loop {
      let start = ip;
      match read_opcode!(ip, end) {
        Opcode::Nop => continue,
        Opcode::Wide16 => {
          width = Width::Wide16;
          continue;
        }
        Opcode::Wide32 => {
          width = Width::Wide32;
          continue;
        }
        Opcode::Load => {
          let (reg,) = read_operands!(Load, ip, end, width);
          handler.op_load(reg).map_err(Error::Handler)?;
          continue;
        }
        Opcode::Store => {
          let (reg,) = read_operands!(Store, ip, end, width);
          handler.op_store(reg).map_err(Error::Handler)?;
          continue;
        }
        Opcode::LoadConst => {
          let (idx,) = read_operands!(LoadConst, ip, end, width);
          handler.op_load_const(idx).map_err(Error::Handler)?;
          continue;
        }
        Opcode::LoadUpvalue => {
          let (idx,) = read_operands!(LoadUpvalue, ip, end, width);
          handler.op_load_upvalue(idx).map_err(Error::Handler)?;
          continue;
        }
        Opcode::StoreUpvalue => {
          let (idx,) = read_operands!(StoreUpvalue, ip, end, width);
          handler.op_store_upvalue(idx).map_err(Error::Handler)?;
          continue;
        }
        Opcode::LoadModuleVar => {
          let (idx,) = read_operands!(LoadModuleVar, ip, end, width);
          handler.op_load_module_var(idx).map_err(Error::Handler)?;
          continue;
        }
        Opcode::StoreModuleVar => {
          let (idx,) = read_operands!(StoreModuleVar, ip, end, width);
          handler.op_store_module_var(idx).map_err(Error::Handler)?;
          continue;
        }
        Opcode::LoadGlobal => {
          let (name,) = read_operands!(LoadGlobal, ip, end, width);
          handler.op_load_global(name).map_err(Error::Handler)?;
          continue;
        }
        Opcode::StoreGlobal => {
          let (name,) = read_operands!(StoreGlobal, ip, end, width);
          handler.op_store_global(name).map_err(Error::Handler)?;
          continue;
        }
        Opcode::LoadField => {
          let (name,) = read_operands!(LoadField, ip, end, width);
          handler.op_load_field(name).map_err(Error::Handler)?;
          continue;
        }
        Opcode::LoadFieldOpt => {
          let (name,) = read_operands!(LoadFieldOpt, ip, end, width);
          handler.op_load_field_opt(name).map_err(Error::Handler)?;
          continue;
        }
        Opcode::StoreField => {
          let (obj, name) = read_operands!(StoreField, ip, end, width);
          handler.op_store_field(obj, name).map_err(Error::Handler)?;
          continue;
        }
        Opcode::LoadIndex => {
          let (name,) = read_operands!(LoadIndex, ip, end, width);
          handler.op_load_index(name).map_err(Error::Handler)?;
          continue;
        }
        Opcode::LoadIndexOpt => {
          let (name,) = read_operands!(LoadIndexOpt, ip, end, width);
          handler.op_load_index_opt(name).map_err(Error::Handler)?;
          continue;
        }
        Opcode::StoreIndex => {
          let (obj, key) = read_operands!(StoreIndex, ip, end, width);
          handler.op_store_index(obj, key).map_err(Error::Handler)?;
          continue;
        }
        Opcode::LoadSelf => {
          let () = read_operands!(LoadSelf, ip, end, width);
          handler.op_load_self().map_err(Error::Handler)?;
          continue;
        }
        Opcode::LoadSuper => {
          let () = read_operands!(LoadSuper, ip, end, width);
          handler.op_load_super().map_err(Error::Handler)?;
          continue;
        }
        Opcode::LoadNone => {
          let () = read_operands!(LoadNone, ip, end, width);
          handler.op_load_none().map_err(Error::Handler)?;
          continue;
        }
        Opcode::LoadTrue => {
          let () = read_operands!(LoadTrue, ip, end, width);
          handler.op_load_true().map_err(Error::Handler)?;
          continue;
        }
        Opcode::LoadFalse => {
          let () = read_operands!(LoadFalse, ip, end, width);
          handler.op_load_false().map_err(Error::Handler)?;
          continue;
        }
        Opcode::LoadSmi => {
          let (smi,) = read_operands!(LoadSmi, ip, end, width);
          handler.op_load_smi(smi).map_err(Error::Handler)?;
          continue;
        }
        Opcode::MakeFn => {
          let (desc,) = read_operands!(MakeFn, ip, end, width);
          handler.op_make_fn(desc).map_err(Error::Handler)?;
          continue;
        }
        Opcode::MakeClassEmpty => {
          let (desc,) = read_operands!(MakeClassEmpty, ip, end, width);
          handler.op_make_class_empty(desc).map_err(Error::Handler)?;
          continue;
        }
        Opcode::MakeClassEmptyDerived => {
          let (desc,) = read_operands!(MakeClassEmptyDerived, ip, end, width);
          handler
            .op_make_class_empty_derived(desc)
            .map_err(Error::Handler)?;
          continue;
        }
        Opcode::MakeClass => {
          let (desc, parts) = read_operands!(MakeClass, ip, end, width);
          handler.op_make_class(desc, parts).map_err(Error::Handler)?;
          continue;
        }
        Opcode::MakeClassDerived => {
          let (desc, parts) = read_operands!(MakeClassDerived, ip, end, width);
          handler
            .op_make_class_derived(desc, parts)
            .map_err(Error::Handler)?;
          continue;
        }
        Opcode::MakeList => {
          let (start, count) = read_operands!(MakeList, ip, end, width);
          handler.op_make_list(start, count).map_err(Error::Handler)?;
          continue;
        }
        Opcode::MakeListEmpty => {
          let () = read_operands!(MakeListEmpty, ip, end, width);
          handler.op_make_list_empty().map_err(Error::Handler)?;
          continue;
        }
        Opcode::MakeTable => {
          let (start, count) = read_operands!(MakeTable, ip, end, width);
          handler
            .op_make_table(start, count)
            .map_err(Error::Handler)?;
          continue;
        }
        Opcode::MakeTableEmpty => {
          let () = read_operands!(MakeTableEmpty, ip, end, width);
          handler.op_make_table_empty().map_err(Error::Handler)?;
          continue;
        }
        Opcode::Jump => {
          #[allow(unused_assignments)] // ip is overwritten by start+offset
          let (offset,) = read_operands!(Jump, ip, end, width);
          let offset = handler.op_jump(offset).map_err(Error::Handler)?;
          unsafe { ip = start.add(offset.value()) };
          continue;
        }
        Opcode::JumpConst => {
          #[allow(unused_assignments)] // ip is overwritten by start+offset
          let (idx,) = read_operands!(JumpConst, ip, end, width);
          let offset = handler.op_jump_const(idx).map_err(Error::Handler)?;
          unsafe { ip = start.add(offset.value()) };
          continue;
        }
        Opcode::JumpLoop => {
          #[allow(unused_assignments)] // ip is overwritten by start-offset
          let (offset,) = read_operands!(JumpLoop, ip, end, width);
          let offset = handler.op_jump_loop(offset).map_err(Error::Handler)?;
          unsafe { ip = start.sub(offset.value()) }
          continue;
        }
        Opcode::JumpIfFalse => {
          let (offset,) = read_operands!(JumpIfFalse, ip, end, width);
          let offset = handler.op_jump_if_false(offset).map_err(Error::Handler)?;
          match offset {
            Jump::Move(offset) => unsafe { ip = start.add(offset.value()) },
            Jump::Skip => {}
          }
          continue;
        }
        Opcode::JumpIfFalseConst => {
          let (idx,) = read_operands!(JumpIfFalseConst, ip, end, width);
          let offset = handler
            .op_jump_if_false_const(idx)
            .map_err(Error::Handler)?;
          match offset {
            Jump::Move(offset) => unsafe { ip = start.add(offset.value()) },
            Jump::Skip => {}
          }
          continue;
        }
        Opcode::Add => {
          let (lhs,) = read_operands!(Add, ip, end, width);
          handler.op_add(lhs).map_err(Error::Handler)?;
          continue;
        }
        Opcode::Sub => {
          let (lhs,) = read_operands!(Sub, ip, end, width);
          handler.op_sub(lhs).map_err(Error::Handler)?;
          continue;
        }
        Opcode::Mul => {
          let (lhs,) = read_operands!(Mul, ip, end, width);
          handler.op_mul(lhs).map_err(Error::Handler)?;
          continue;
        }
        Opcode::Div => {
          let (lhs,) = read_operands!(Div, ip, end, width);
          handler.op_div(lhs).map_err(Error::Handler)?;
          continue;
        }
        Opcode::Rem => {
          let (lhs,) = read_operands!(Rem, ip, end, width);
          handler.op_rem(lhs).map_err(Error::Handler)?;
          continue;
        }
        Opcode::Pow => {
          let (lhs,) = read_operands!(Pow, ip, end, width);
          handler.op_pow(lhs).map_err(Error::Handler)?;
          continue;
        }
        Opcode::Inv => {
          let () = read_operands!(Inv, ip, end, width);
          handler.op_inv().map_err(Error::Handler)?;
          continue;
        }
        Opcode::Not => {
          let () = read_operands!(Not, ip, end, width);
          handler.op_not().map_err(Error::Handler)?;
          continue;
        }
        Opcode::CmpEq => {
          let (lhs,) = read_operands!(CmpEq, ip, end, width);
          handler.op_cmp_eq(lhs).map_err(Error::Handler)?;
          continue;
        }
        Opcode::CmpNe => {
          let (lhs,) = read_operands!(CmpNe, ip, end, width);
          handler.op_cmp_ne(lhs).map_err(Error::Handler)?;
          continue;
        }
        Opcode::CmpGt => {
          let (lhs,) = read_operands!(CmpGt, ip, end, width);
          handler.op_cmp_gt(lhs).map_err(Error::Handler)?;
          continue;
        }
        Opcode::CmpGe => {
          let (lhs,) = read_operands!(CmpGe, ip, end, width);
          handler.op_cmp_ge(lhs).map_err(Error::Handler)?;
          continue;
        }
        Opcode::CmpLt => {
          let (lhs,) = read_operands!(CmpLt, ip, end, width);
          handler.op_cmp_lt(lhs).map_err(Error::Handler)?;
          continue;
        }
        Opcode::CmpLe => {
          let (lhs,) = read_operands!(CmpLe, ip, end, width);
          handler.op_cmp_le(lhs).map_err(Error::Handler)?;
          continue;
        }
        Opcode::CmpType => {
          let (lhs,) = read_operands!(CmpType, ip, end, width);
          handler.op_cmp_type(lhs).map_err(Error::Handler)?;
          continue;
        }
        Opcode::Contains => {
          let (lhs,) = read_operands!(Contains, ip, end, width);
          handler.op_contains(lhs).map_err(Error::Handler)?;
          continue;
        }
        Opcode::IsNone => {
          let () = read_operands!(IsNone, ip, end, width);
          handler.op_is_none().map_err(Error::Handler)?;
          continue;
        }
        Opcode::Print => {
          let () = read_operands!(Print, ip, end, width);
          handler.op_print().map_err(Error::Handler)?;
          continue;
        }
        Opcode::PrintN => {
          let (start, count) = read_operands!(PrintN, ip, end, width);
          handler.op_print_n(start, count).map_err(Error::Handler)?;
          continue;
        }
        Opcode::Call => {
          // frame is reloaded so neither `ip` nor `width` are read
          #[allow(unused_assignments)]
          let (callee, args) = read_operands!(Call, ip, end, width);
          let return_addr = get_pc!(ip, bytecode);
          let new_frame = handler
            .op_call(return_addr, callee, args)
            .map_err(Error::Handler)?;
          bytecode = new_frame.bytecode;
          pc = new_frame.pc;
          continue 'load_frame;
        }
        Opcode::Call0 => {
          // frame is reloaded so neither `ip` nor `width` are read
          #[allow(unused_assignments)]
          let () = read_operands!(Call0, ip, end, width);
          let return_addr = get_pc!(ip, bytecode);
          let new_frame = handler.op_call0(return_addr).map_err(Error::Handler)?;
          bytecode = new_frame.bytecode;
          pc = new_frame.pc;
          continue 'load_frame;
        }
        Opcode::Import => {
          let (path, dst) = read_operands!(Import, ip, end, width);
          handler.op_import(path, dst).map_err(Error::Handler)?;
          continue;
        }
        Opcode::Return => {
          #[allow(unused_assignments)] // ip is overwritten by start+offset
          let () = read_operands!(Return, ip, end, width);
          match handler.op_return().map_err(Error::Handler)? {
            Return::LoadFrame(new_frame) => {
              bytecode = new_frame.bytecode;
              pc = new_frame.pc;
              continue 'load_frame;
            }
            Return::Yield => return Ok(ControlFlow::Return),
          };
        }
        Opcode::Yield => {
          #[allow(unused_assignments)] // ip is overwritten by start+offset
          let () = read_operands!(Yield, ip, end, width);
          handler.op_yield().map_err(Error::Handler)?;

          return Ok(ControlFlow::Yield(
            (ip as usize) - (bytecode.as_ptr() as *mut u8 as usize),
          ));
        }
      }
    }
  }
}

pub enum ControlFlow {
  Return,
  Yield(usize),
}

pub enum Jump {
  Skip,
  Move(op::Offset),
}

pub struct LoadFrame {
  pub bytecode: NonNull<[u8]>,
  pub pc: usize,
}

pub enum Return {
  LoadFrame(LoadFrame),
  Yield,
}

#[derive(Debug)]
pub enum Error<Inner: StdError> {
  IllegalInstruction,
  UnexpectedEnd,
  Handler(Inner),
}

impl<Inner: StdError> Display for Error<Inner> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Error::IllegalInstruction => write!(f, "illegal instruction"),
      Error::UnexpectedEnd => write!(f, "unexpected end of bytecode stream"),
      Error::Handler(e) => write!(f, "{e}"),
    }
  }
}

impl<Inner: StdError> StdError for Error<Inner> {}

pub trait Handler {
  type Error: StdError;

  fn op_load(&mut self, reg: op::Register) -> Result<(), Self::Error>;
  fn op_store(&mut self, reg: op::Register) -> Result<(), Self::Error>;
  fn op_load_const(&mut self, idx: op::Constant) -> Result<(), Self::Error>;
  fn op_load_upvalue(&mut self, idx: op::Upvalue) -> Result<(), Self::Error>;
  fn op_store_upvalue(&mut self, idx: op::Upvalue) -> Result<(), Self::Error>;
  fn op_load_module_var(&mut self, idx: op::ModuleVar) -> Result<(), Self::Error>;
  fn op_store_module_var(&mut self, idx: op::ModuleVar) -> Result<(), Self::Error>;
  fn op_load_global(&mut self, name: op::Constant) -> Result<(), Self::Error>;
  fn op_store_global(&mut self, name: op::Constant) -> Result<(), Self::Error>;
  fn op_load_field(&mut self, name: op::Constant) -> Result<(), Self::Error>;
  fn op_load_field_opt(&mut self, name: op::Constant) -> Result<(), Self::Error>;
  fn op_store_field(&mut self, obj: op::Register, name: op::Constant) -> Result<(), Self::Error>;
  fn op_load_index(&mut self, obj: op::Register) -> Result<(), Self::Error>;
  fn op_load_index_opt(&mut self, obj: op::Register) -> Result<(), Self::Error>;
  fn op_store_index(&mut self, obj: op::Register, key: op::Register) -> Result<(), Self::Error>;
  fn op_load_self(&mut self) -> Result<(), Self::Error>;
  fn op_load_super(&mut self) -> Result<(), Self::Error>;
  fn op_load_none(&mut self) -> Result<(), Self::Error>;
  fn op_load_true(&mut self) -> Result<(), Self::Error>;
  fn op_load_false(&mut self) -> Result<(), Self::Error>;
  fn op_load_smi(&mut self, smi: op::Smi) -> Result<(), Self::Error>;
  fn op_make_fn(&mut self, desc: op::Constant) -> Result<(), Self::Error>;
  fn op_make_class_empty(&mut self, desc: op::Constant) -> Result<(), Self::Error>;
  fn op_make_class_empty_derived(&mut self, desc: op::Constant) -> Result<(), Self::Error>;
  fn op_make_class(&mut self, desc: op::Constant, parts: op::Register) -> Result<(), Self::Error>;
  fn op_make_class_derived(
    &mut self,
    desc: op::Constant,
    parts: op::Register,
  ) -> Result<(), Self::Error>;
  fn op_make_list(&mut self, start: op::Register, count: op::Count) -> Result<(), Self::Error>;
  fn op_make_list_empty(&mut self) -> Result<(), Self::Error>;
  fn op_make_table(&mut self, start: op::Register, count: op::Count) -> Result<(), Self::Error>;
  fn op_make_table_empty(&mut self) -> Result<(), Self::Error>;
  fn op_jump(&mut self, offset: op::Offset) -> Result<op::Offset, Self::Error>;
  fn op_jump_const(&mut self, idx: op::Constant) -> Result<op::Offset, Self::Error>;
  fn op_jump_loop(&mut self, offset: op::Offset) -> Result<op::Offset, Self::Error>;
  fn op_jump_if_false(&mut self, offset: op::Offset) -> Result<Jump, Self::Error>;
  fn op_jump_if_false_const(&mut self, idx: op::Constant) -> Result<Jump, Self::Error>;
  fn op_add(&mut self, lhs: op::Register) -> Result<(), Self::Error>;
  fn op_sub(&mut self, lhs: op::Register) -> Result<(), Self::Error>;
  fn op_mul(&mut self, lhs: op::Register) -> Result<(), Self::Error>;
  fn op_div(&mut self, lhs: op::Register) -> Result<(), Self::Error>;
  fn op_rem(&mut self, lhs: op::Register) -> Result<(), Self::Error>;
  fn op_pow(&mut self, lhs: op::Register) -> Result<(), Self::Error>;
  fn op_inv(&mut self) -> Result<(), Self::Error>;
  fn op_not(&mut self) -> Result<(), Self::Error>;
  fn op_cmp_eq(&mut self, lhs: op::Register) -> Result<(), Self::Error>;
  fn op_cmp_ne(&mut self, lhs: op::Register) -> Result<(), Self::Error>;
  fn op_cmp_gt(&mut self, lhs: op::Register) -> Result<(), Self::Error>;
  fn op_cmp_ge(&mut self, lhs: op::Register) -> Result<(), Self::Error>;
  fn op_cmp_lt(&mut self, lhs: op::Register) -> Result<(), Self::Error>;
  fn op_cmp_le(&mut self, lhs: op::Register) -> Result<(), Self::Error>;
  fn op_cmp_type(&mut self, lhs: op::Register) -> Result<(), Self::Error>;
  fn op_contains(&mut self, lhs: op::Register) -> Result<(), Self::Error>;
  fn op_is_none(&mut self) -> Result<(), Self::Error>;
  fn op_print(&mut self) -> Result<(), Self::Error>;
  fn op_print_n(&mut self, start: op::Register, count: op::Count) -> Result<(), Self::Error>;
  fn op_call(
    &mut self,
    return_addr: usize,
    callee: op::Register,
    args: op::Count,
  ) -> Result<LoadFrame, Self::Error>;
  fn op_call0(&mut self, return_addr: usize) -> Result<LoadFrame, Self::Error>;
  fn op_import(&mut self, path: op::Constant, dst: op::Register) -> Result<(), Self::Error>;
  fn op_return(&mut self) -> Result<Return, Self::Error>;
  fn op_yield(&mut self) -> Result<(), Self::Error>;
}
