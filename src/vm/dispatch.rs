#[macro_use]
mod macros;

use std::error::Error as StdError;

use crate::bytecode::opcode::Opcode;
use crate::bytecode::operands::Width;

pub fn dispatch<T: Handler>(
  handler: &T,
  bytecode: &[u8],
  pc: usize,
) -> Result<ControlFlow, Error<T::Error>> {
  let mut pc = pc;
  let mut width = Width::Normal;

  loop {
    let opcode = bytecode
      .get(pc)
      .copied()
      .ok_or_else(|| Error::UnexpectedEnd)?;
    let opcode = Opcode::try_from(opcode).map_err(|_| Error::IllegalInstruction)?;
    match opcode {
      Opcode::Nop => {
        pc += 1;
        width = Width::Normal;
      }
      Opcode::Wide16 => {
        pc += 1;
        width = Width::Wide16;
      }
      Opcode::Wide32 => {
        pc += 1;
        width = Width::Wide32;
      }
      Opcode::Load => {
        let (reg,) = operands!(Load, bytecode, pc, width);
        handler.op_load(reg as usize).map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(Load);
        width = Width::Normal;
      }
      Opcode::Store => {
        let (reg,) = operands!(Store, bytecode, pc, width);
        handler.op_store(reg as usize).map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(Store);
        width = Width::Normal;
      }
      Opcode::LoadConst => {
        let (idx,) = operands!(LoadConst, bytecode, pc, width);
        handler
          .op_load_const(idx as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(LoadConst);
        width = Width::Normal;
      }
      Opcode::LoadUpvalue => {
        let (idx,) = operands!(LoadUpvalue, bytecode, pc, width);
        handler
          .op_load_upvalue(idx as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(LoadUpvalue);
        width = Width::Normal;
      }
      Opcode::StoreUpvalue => {
        let (idx,) = operands!(StoreUpvalue, bytecode, pc, width);
        handler
          .op_store_upvalue(idx as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(StoreUpvalue);
        width = Width::Normal;
      }
      Opcode::LoadModuleVar => {
        let (idx,) = operands!(LoadModuleVar, bytecode, pc, width);
        handler
          .op_load_module_var(idx as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(LoadModuleVar);
        width = Width::Normal;
      }
      Opcode::StoreModuleVar => {
        let (idx,) = operands!(StoreModuleVar, bytecode, pc, width);
        handler
          .op_store_module_var(idx as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(StoreModuleVar);
        width = Width::Normal;
      }
      Opcode::LoadGlobal => {
        let (name,) = operands!(LoadGlobal, bytecode, pc, width);
        handler
          .op_load_global(name as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(LoadGlobal);
        width = Width::Normal;
      }
      Opcode::StoreGlobal => {
        let (name,) = operands!(StoreGlobal, bytecode, pc, width);
        handler
          .op_store_global(name as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(StoreGlobal);
        width = Width::Normal;
      }
      Opcode::LoadField => {
        let (name,) = operands!(LoadField, bytecode, pc, width);
        handler
          .op_load_field(name as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(LoadField);
        width = Width::Normal;
      }
      Opcode::LoadFieldOpt => {
        let (name,) = operands!(LoadFieldOpt, bytecode, pc, width);
        handler
          .op_load_field_opt(name as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(LoadFieldOpt);
        width = Width::Normal;
      }
      Opcode::StoreField => {
        let (obj, name) = operands!(StoreField, bytecode, pc, width);
        handler
          .op_store_field(obj as usize, name as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(StoreField);
        width = Width::Normal;
      }
      Opcode::LoadIndex => {
        let (name,) = operands!(LoadIndex, bytecode, pc, width);
        handler
          .op_load_index(name as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(LoadIndex);
        width = Width::Normal;
      }
      Opcode::LoadIndexOpt => {
        let (name,) = operands!(LoadIndexOpt, bytecode, pc, width);
        handler
          .op_load_index_opt(name as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(LoadIndexOpt);
        width = Width::Normal;
      }
      Opcode::StoreIndex => {
        let (obj, key) = operands!(StoreIndex, bytecode, pc, width);
        handler
          .op_store_index(obj as usize, key as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(StoreIndex);
        width = Width::Normal;
      }
      Opcode::LoadSelf => {
        handler.op_load_self().map_err(Error::Handler)?;

        pc += 1;
        width = Width::Normal;
      }
      Opcode::LoadSuper => {
        handler.op_load_super().map_err(Error::Handler)?;

        pc += 1;
        width = Width::Normal;
      }
      Opcode::LoadNone => {
        handler.op_load_none().map_err(Error::Handler)?;

        pc += 1;
        width = Width::Normal;
      }
      Opcode::LoadTrue => {
        handler.op_load_true().map_err(Error::Handler)?;

        pc += 1;
        width = Width::Normal;
      }
      Opcode::LoadFalse => {
        handler.op_load_false().map_err(Error::Handler)?;

        pc += 1;
        width = Width::Normal;
      }
      Opcode::LoadSmi => {
        let (value,) = operands!(LoadSmi, bytecode, pc, width);
        handler.op_load_smi(value).map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(LoadSmi);
        width = Width::Normal;
      }
      Opcode::MakeFn => {
        let (desc,) = operands!(MakeFn, bytecode, pc, width);
        handler.op_make_fn(desc as usize).map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(MakeFn);
        width = Width::Normal;
      }
      Opcode::MakeClassEmpty => {
        let (desc,) = operands!(MakeClassEmpty, bytecode, pc, width);
        handler
          .op_make_class_empty(desc as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(MakeClassEmpty);
        width = Width::Normal;
      }
      Opcode::MakeClassEmptyDerived => {
        let (desc,) = operands!(MakeClassEmptyDerived, bytecode, pc, width);
        handler
          .op_make_class_empty_derived(desc as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(MakeClassEmptyDerived);
        width = Width::Normal;
      }
      Opcode::MakeClass => {
        let (desc, parts) = operands!(MakeClass, bytecode, pc, width);
        handler
          .op_make_class(desc as usize, parts as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(MakeClass);
        width = Width::Normal;
      }
      Opcode::MakeClassDerived => {
        let (desc, parts) = operands!(MakeClassDerived, bytecode, pc, width);
        handler
          .op_make_class_derived(desc as usize, parts as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(MakeClassDerived);
        width = Width::Normal;
      }
      Opcode::MakeList => {
        let (start, count) = operands!(MakeList, bytecode, pc, width);
        handler
          .op_make_list(start as usize, count as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(MakeList);
        width = Width::Normal;
      }
      Opcode::MakeListEmpty => {
        handler.op_make_list_empty().map_err(Error::Handler)?;

        pc += 1;
        width = Width::Normal;
      }
      Opcode::MakeTable => {
        let (start, count) = operands!(MakeTable, bytecode, pc, width);
        handler
          .op_make_table(start as usize, count as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(MakeTable);
        width = Width::Normal;
      }
      Opcode::MakeTableEmpty => {
        handler.op_make_table_empty().map_err(Error::Handler)?;

        pc += 1;
        width = Width::Normal;
      }
      Opcode::Jump => {
        let (offset,) = operands!(Jump, bytecode, pc, width);
        let offset = handler.op_jump(offset as usize).map_err(Error::Handler)?;

        pc += offset;
        width = Width::Normal;
      }
      Opcode::JumpConst => {
        let (idx,) = operands!(JumpConst, bytecode, pc, width);
        let offset = handler
          .op_jump_const(idx as usize)
          .map_err(Error::Handler)?;

        pc += offset;
        width = Width::Normal;
      }
      Opcode::JumpLoop => {
        let (offset,) = operands!(JumpLoop, bytecode, pc, width);
        let offset = handler
          .op_jump_loop(offset as usize)
          .map_err(Error::Handler)?;

        pc -= offset;
        width = Width::Normal;
      }
      Opcode::JumpIfFalse => {
        let (offset,) = operands!(JumpIfFalse, bytecode, pc, width);
        let offset = handler
          .op_jump_if_false(offset as usize)
          .map_err(Error::Handler)?;

        match offset {
          Jump::Move(offset) => pc += offset,
          Jump::Skip => pc += 1 + size_of_operands!(JumpIfFalse),
        }
        width = Width::Normal;
      }
      Opcode::JumpIfFalseConst => {
        let (idx,) = operands!(JumpIfFalseConst, bytecode, pc, width);
        let offset = handler
          .op_jump_if_false_const(idx as usize)
          .map_err(Error::Handler)?;

        match offset {
          Jump::Move(offset) => pc += offset,
          Jump::Skip => pc += 1 + size_of_operands!(JumpIfFalseConst),
        }
        width = Width::Normal;
      }
      Opcode::Add => {
        let (lhs,) = operands!(Add, bytecode, pc, width);
        handler.op_add(lhs as usize).map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(Add);
        width = Width::Normal;
      }
      Opcode::Sub => {
        let (lhs,) = operands!(Sub, bytecode, pc, width);
        handler.op_sub(lhs as usize).map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(Sub);
        width = Width::Normal;
      }
      Opcode::Mul => {
        let (lhs,) = operands!(Mul, bytecode, pc, width);
        handler.op_mul(lhs as usize).map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(Mul);
        width = Width::Normal;
      }
      Opcode::Div => {
        let (lhs,) = operands!(Div, bytecode, pc, width);
        handler.op_div(lhs as usize).map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(Div);
        width = Width::Normal;
      }
      Opcode::Rem => {
        let (lhs,) = operands!(Rem, bytecode, pc, width);
        handler.op_rem(lhs as usize).map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(Rem);
        width = Width::Normal;
      }
      Opcode::Pow => {
        let (lhs,) = operands!(Pow, bytecode, pc, width);
        handler.op_pow(lhs as usize).map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(Pow);
        width = Width::Normal;
      }
      Opcode::Inv => {
        handler.op_inv().map_err(Error::Handler)?;

        pc += 1;
        width = Width::Normal;
      }
      Opcode::Not => {
        handler.op_not().map_err(Error::Handler)?;

        pc += 1;
        width = Width::Normal;
      }
      Opcode::CmpEq => {
        let (lhs,) = operands!(CmpEq, bytecode, pc, width);
        handler.op_cmp_eq(lhs as usize).map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(CmpEq);
        width = Width::Normal;
      }
      Opcode::CmpNe => {
        let (lhs,) = operands!(CmpNe, bytecode, pc, width);
        handler.op_cmp_ne(lhs as usize).map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(CmpNe);
        width = Width::Normal;
      }
      Opcode::CmpGt => {
        let (lhs,) = operands!(CmpGt, bytecode, pc, width);
        handler.op_cmp_gt(lhs as usize).map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(CmpGt);
        width = Width::Normal;
      }
      Opcode::CmpGe => {
        let (lhs,) = operands!(CmpGe, bytecode, pc, width);
        handler.op_cmp_ge(lhs as usize).map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(CmpGe);
        width = Width::Normal;
      }
      Opcode::CmpLt => {
        let (lhs,) = operands!(CmpLt, bytecode, pc, width);
        handler.op_cmp_lt(lhs as usize).map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(CmpLt);
        width = Width::Normal;
      }
      Opcode::CmpLe => {
        let (lhs,) = operands!(CmpLe, bytecode, pc, width);
        handler.op_cmp_le(lhs as usize).map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(CmpLe);
        width = Width::Normal;
      }
      Opcode::CmpType => {
        let (lhs,) = operands!(CmpType, bytecode, pc, width);
        handler.op_cmp_type(lhs as usize).map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(CmpType);
        width = Width::Normal;
      }
      Opcode::Contains => {
        let (lhs,) = operands!(Contains, bytecode, pc, width);
        handler.op_contains(lhs as usize).map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(Contains);
        width = Width::Normal;
      }
      Opcode::IsNone => {
        handler.op_is_none().map_err(Error::Handler)?;

        pc += 1;
        width = Width::Normal;
      }
      Opcode::Print => {
        handler.op_print().map_err(Error::Handler)?;

        pc += 1;
        width = Width::Normal;
      }
      Opcode::PrintN => {
        let (start, count) = operands!(PrintN, bytecode, pc, width);
        handler
          .op_print_n(start as usize, count as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(PrintN);
        width = Width::Normal;
      }
      Opcode::Call => {
        let (callee, args) = operands!(Call, bytecode, pc, width);
        handler
          .op_print_n(callee as usize, args as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(Call);
        width = Width::Normal;
      }
      Opcode::Call0 => {
        handler.op_call0().map_err(Error::Handler)?;

        pc += 1;
        width = Width::Normal;
      }
      Opcode::Import => {
        let (path, dst) = operands!(Import, bytecode, pc, width);
        handler
          .op_import(path as usize, dst as usize)
          .map_err(Error::Handler)?;

        pc += 1 + size_of_operands!(Import);
        width = Width::Normal;
      }
      Opcode::Return => {
        handler.op_return().map_err(Error::Handler)?;

        return Ok(ControlFlow::Return);
      }
      Opcode::Yield => {
        handler.op_yield().map_err(Error::Handler)?;

        return Ok(ControlFlow::Yield(pc));
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
  Move(usize),
}

pub enum Error<E: StdError> {
  IllegalInstruction,
  UnexpectedEnd,
  Handler(E),
}

pub trait Handler {
  type Error: StdError;

  fn op_load(&self, reg: usize) -> Result<(), Self::Error>;
  fn op_store(&self, reg: usize) -> Result<(), Self::Error>;
  fn op_load_const(&self, idx: usize) -> Result<(), Self::Error>;
  fn op_load_upvalue(&self, idx: usize) -> Result<(), Self::Error>;
  fn op_store_upvalue(&self, idx: usize) -> Result<(), Self::Error>;
  fn op_load_module_var(&self, idx: usize) -> Result<(), Self::Error>;
  fn op_store_module_var(&self, idx: usize) -> Result<(), Self::Error>;
  fn op_load_global(&self, name: usize) -> Result<(), Self::Error>;
  fn op_store_global(&self, name: usize) -> Result<(), Self::Error>;
  fn op_load_field(&self, name: usize) -> Result<(), Self::Error>;
  fn op_load_field_opt(&self, name: usize) -> Result<(), Self::Error>;
  fn op_store_field(&self, obj: usize, name: usize) -> Result<(), Self::Error>;
  fn op_load_index(&self, obj: usize) -> Result<(), Self::Error>;
  fn op_load_index_opt(&self, obj: usize) -> Result<(), Self::Error>;
  fn op_store_index(&self, obj: usize, key: usize) -> Result<(), Self::Error>;
  fn op_load_self(&self) -> Result<(), Self::Error>;
  fn op_load_super(&self) -> Result<(), Self::Error>;
  fn op_load_none(&self) -> Result<(), Self::Error>;
  fn op_load_true(&self) -> Result<(), Self::Error>;
  fn op_load_false(&self) -> Result<(), Self::Error>;
  fn op_load_smi(&self, value: i32) -> Result<(), Self::Error>;
  fn op_make_fn(&self, desc: usize) -> Result<(), Self::Error>;
  fn op_make_class_empty(&self, desc: usize) -> Result<(), Self::Error>;
  fn op_make_class_empty_derived(&self, desc: usize) -> Result<(), Self::Error>;
  fn op_make_class(&self, desc: usize, parts: usize) -> Result<(), Self::Error>;
  fn op_make_class_derived(&self, desc: usize, parts: usize) -> Result<(), Self::Error>;
  fn op_make_list(&self, start: usize, count: usize) -> Result<(), Self::Error>;
  fn op_make_list_empty(&self) -> Result<(), Self::Error>;
  fn op_make_table(&self, start: usize, count: usize) -> Result<(), Self::Error>;
  fn op_make_table_empty(&self) -> Result<(), Self::Error>;
  fn op_jump(&self, offset: usize) -> Result<usize, Self::Error>;
  fn op_jump_const(&self, idx: usize) -> Result<usize, Self::Error>;
  fn op_jump_loop(&self, offset: usize) -> Result<usize, Self::Error>;
  fn op_jump_if_false(&self, offset: usize) -> Result<Jump, Self::Error>;
  fn op_jump_if_false_const(&self, idx: usize) -> Result<Jump, Self::Error>;
  fn op_add(&self, lhs: usize) -> Result<(), Self::Error>;
  fn op_sub(&self, lhs: usize) -> Result<(), Self::Error>;
  fn op_mul(&self, lhs: usize) -> Result<(), Self::Error>;
  fn op_div(&self, lhs: usize) -> Result<(), Self::Error>;
  fn op_rem(&self, lhs: usize) -> Result<(), Self::Error>;
  fn op_pow(&self, lhs: usize) -> Result<(), Self::Error>;
  fn op_inv(&self) -> Result<(), Self::Error>;
  fn op_not(&self) -> Result<(), Self::Error>;
  fn op_cmp_eq(&self, lhs: usize) -> Result<(), Self::Error>;
  fn op_cmp_ne(&self, lhs: usize) -> Result<(), Self::Error>;
  fn op_cmp_gt(&self, lhs: usize) -> Result<(), Self::Error>;
  fn op_cmp_ge(&self, lhs: usize) -> Result<(), Self::Error>;
  fn op_cmp_lt(&self, lhs: usize) -> Result<(), Self::Error>;
  fn op_cmp_le(&self, lhs: usize) -> Result<(), Self::Error>;
  fn op_cmp_type(&self, lhs: usize) -> Result<(), Self::Error>;
  fn op_contains(&self, lhs: usize) -> Result<(), Self::Error>;
  fn op_is_none(&self) -> Result<(), Self::Error>;
  fn op_print(&self) -> Result<(), Self::Error>;
  fn op_print_n(&self, start: usize, count: usize) -> Result<(), Self::Error>;
  fn op_call(&self, callee: usize, args: usize) -> Result<(), Self::Error>;
  fn op_call0(&self) -> Result<(), Self::Error>;
  fn op_import(&self, path: usize, dst: usize) -> Result<(), Self::Error>;
  fn op_return(&self) -> Result<(), Self::Error>;
  fn op_yield(&self) -> Result<(), Self::Error>;
}
