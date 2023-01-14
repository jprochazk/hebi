use crate::chunk::BytecodeArray;
use crate::disassembly::disassemble;
use crate::handler;
use crate::handler::Handler;
use crate::opcode::ty::Width;
use crate::opcode::{self as op, ops, Decode, Opcode};

#[inline(never)]
pub fn run<H: Handler>(vm: &mut H, bc: &mut BytecodeArray, pc: &mut usize) -> Result<(), H::Error> {
  let opcode = &mut (bc[*pc].clone());
  let width = &mut Width::Single;
  let mut result = Ok(());
  while result.is_ok() {
    let result = &mut result;
    match *opcode {
      ops::Nop => op_nop(vm, bc, pc, opcode, width, result),
      ops::Wide => op_wide(vm, bc, pc, opcode, width, result),
      ops::ExtraWide => op_extra_wide(vm, bc, pc, opcode, width, result),
      ops::LoadConst => op_load_const(vm, bc, pc, opcode, width, result),
      ops::LoadReg => op_load_reg(vm, bc, pc, opcode, width, result),
      ops::StoreReg => op_store_reg(vm, bc, pc, opcode, width, result),
      ops::Jump => op_jump(vm, bc, pc, opcode, width, result),
      ops::JumpIfFalse => op_jump_if_false(vm, bc, pc, opcode, width, result),
      ops::Sub => op_sub(vm, bc, pc, opcode, width, result),
      ops::Print => op_print(vm, bc, pc, opcode, width, result),
      ops::PushSmallInt => op_push_small_int(vm, bc, pc, opcode, width, result),
      ops::CreateEmptyList => op_create_empty_list(vm, bc, pc, opcode, width, result),
      ops::ListPush => op_list_push(vm, bc, pc, opcode, width, result),
      ops::Ret => op_ret(vm, bc, pc, opcode, width, result),
      ops::Suspend => break,
      _ => panic!("malformed bytecode: invalid opcode {}", *opcode),
    }
  }
  result
}

fn handle_jump<E>(
  value: Result<handler::ControlFlow, E>,
  pc: &mut usize,
  size_of_operands: usize,
  result: &mut Result<(), E>,
) {
  let _jump = match value {
    Ok(jump) => jump,
    Err(e) => {
      *result = Err(e);
      handler::ControlFlow::Next
    }
  };
  match _jump {
    handler::ControlFlow::Next => *pc += 1 + size_of_operands,
    handler::ControlFlow::Goto(offset) => *pc = offset as usize,
  }
}

macro_rules! disassemble {
  ($bc:ident, $pc:ident) => {
    if cfg!(feature = "disassembly") {
      println!("{}", disassemble($bc, *$pc));
    }
  };
}

macro_rules! dispatch_handler {
  ($name:ident, $op:ident, :jump) => {
    #[inline]
    fn $name<H: Handler>(
      vm: &mut H,
      bc: &mut BytecodeArray,
      pc: &mut usize,
      opcode: &mut u8,
      width: &mut Width,
      result: &mut Result<(), H::Error>,
    ) {
      #[allow(dead_code)]
      const fn assert_is_jump() {
        if !op::$op::IS_JUMP {
          panic!("not a jump instruction");
        }
      }
      const _: () = assert_is_jump();

      disassemble!(bc, pc);
      handle_jump(
        vm.$name(op::$op::decode(bc, *pc + 1, *width)),
        pc,
        op::$op::size_of_operands(*width),
        result
      );
      *width = Width::Single;
      *opcode = bc[*pc];
    }
  };
  ($name:ident, $op:ident) => {
    dispatch_handler!($name, $op, skip_vm: false, next_width: Single);
  };
  ($name:ident, $op:ident, skip_vm:$skip_vm:literal) => {
    dispatch_handler!($name, $op, skip_vm: $skip_vm, next_width: Single);
  };
  ($name:ident, $op:ident, skip_vm:false, next_width:$next_width:ident) => {
    #[inline]
    fn $name<H: Handler>(
      vm: &mut H,
      bc: &mut BytecodeArray,
      pc: &mut usize,
      opcode: &mut u8,
      width: &mut Width,
      result: &mut Result<(), H::Error>,
    ) {
      disassemble!(bc, pc);
      *result = vm.$name(op::$op::decode(bc, *pc + 1, *width));
      *pc += 1 + op::$op::size_of_operands(*width);
      *width = Width::$next_width;
      *opcode = bc[*pc];
    }
  };
  ($name:ident, $op:ident, skip_vm:true, next_width:$next_width:ident) => {
    #[inline]
    fn $name<H: Handler>(
      _: &mut H,
      bc: &mut BytecodeArray,
      pc: &mut usize,
      opcode: &mut u8,
      width: &mut Width,
      _: &mut Result<(), H::Error>,
    ) {
      disassemble!(bc, pc);
      *pc += 1 + op::$op::size_of_operands(*width);
      *width = Width::$next_width;
      *opcode = bc[*pc];
    }
  }
}

dispatch_handler!(op_nop, Nop, skip_vm:true, next_width: Single);
dispatch_handler!(op_wide, Wide, skip_vm:true, next_width:Double);
dispatch_handler!(op_extra_wide, ExtraWide, skip_vm:true, next_width:Quad);
dispatch_handler!(op_load_const, LoadConst);
dispatch_handler!(op_load_reg, LoadReg);
dispatch_handler!(op_store_reg, StoreReg);
dispatch_handler!(op_jump, Jump, :jump);
dispatch_handler!(op_jump_if_false, JumpIfFalse, :jump);
dispatch_handler!(op_sub, Sub);
dispatch_handler!(op_print, Print);
dispatch_handler!(op_push_small_int, PushSmallInt);
dispatch_handler!(op_create_empty_list, CreateEmptyList);
dispatch_handler!(op_list_push, ListPush);
dispatch_handler!(op_ret, Ret);
