use super::emit::builder::JumpOffset;
use super::{Capture, Const, Count, Mvar, Offset, Op, Reg, Smi};

macro_rules! _asm {
  ($inst:ident $(, $($arg:ident : $ty:ident<$g:ident>),*)?) => {
    #[allow(non_camel_case_types)]
    pub fn $inst ($($($arg : $ty<impl Into<$g>>),*)?) -> Op {
      $( $( let $arg = $ty($arg.0.into()); )* )?

      paste::paste!(
        Op::[<$inst:camel>] $({
          $($arg),*
        })?
      )
    }
  };
}

// TODO: prune unused ops

_asm!(nop);
_asm!(mov,                   src: Reg<u8>, dst: Reg<u8>);
_asm!(load_const,            dst: Reg<u8>, idx: Const<u16>);
_asm!(load_capture,          dst: Reg<u8>, idx: Capture<u16>);
_asm!(store_capture,         src: Reg<u8>, idx: Capture<u16>);
_asm!(load_mvar,             dst: Reg<u8>, idx: Mvar<u16>);
_asm!(store_mvar,            src: Reg<u8>, idx: Mvar<u16>);
_asm!(load_global,           dst: Reg<u8>, key: Const<u16>);
_asm!(store_global,          src: Reg<u8>, key: Const<u16>);
_asm!(load_field,            obj: Reg<u8>, key: Const<u8>, dst: Reg<u8>);
_asm!(load_field_r,          obj: Reg<u8>, key: Reg<u8>, dst: Reg<u8>);
_asm!(load_field_opt,        obj: Reg<u8>, key: Const<u8>, dst: Reg<u8>);
_asm!(load_field_r_opt,      obj: Reg<u8>, key: Reg<u8>, dst: Reg<u8>);
_asm!(load_field_int,        obj: Reg<u8>, key: Const<u8>, dst: Reg<u8>);
_asm!(load_field_int_r,      obj: Reg<u8>, key: Reg<u8>, dst: Reg<u8>);
_asm!(load_field_int_opt,    obj: Reg<u8>, key: Const<u8>, dst: Reg<u8>);
_asm!(load_field_int_r_opt,  obj: Reg<u8>, key: Reg<u8>, dst: Reg<u8>);
_asm!(store_field,           obj: Reg<u8>, key: Const<u8>, src: Reg<u8>);
_asm!(store_field_r,         obj: Reg<u8>, key: Reg<u8>, src: Reg<u8>);
_asm!(store_field_int,       obj: Reg<u8>, key: Const<u8>, src: Reg<u8>);
_asm!(store_field_int_r,     obj: Reg<u8>, key: Reg<u8>, src: Reg<u8>);
_asm!(load_index,            obj: Reg<u8>, key: Reg<u8>, dst: Reg<u8>);
_asm!(load_index_opt,        obj: Reg<u8>, key: Reg<u8>, dst: Reg<u8>);
_asm!(store_index,           obj: Reg<u8>, key: Reg<u8>, src: Reg<u8>);
_asm!(load_super,            dst: Reg<u8>);
_asm!(load_nil,              dst: Reg<u8>);
_asm!(load_true,             dst: Reg<u8>);
_asm!(load_false,            dst: Reg<u8>);
_asm!(load_smi,              dst: Reg<u8>, value: Smi<i16>);
_asm!(make_fn,               dst: Reg<u8>, desc: Const<u16>);
_asm!(make_class,            dst: Reg<u8>, desc: Const<u16>);
_asm!(make_class_derived,    dst: Reg<u8>, desc: Const<u16>);
_asm!(make_list,             dst: Reg<u8>, desc: Const<u16>);
_asm!(make_list_empty,       dst: Reg<u8>);
_asm!(make_map,              dst: Reg<u8>, desc: Const<u16>);
_asm!(make_map_empty,        dst: Reg<u8>);
_asm!(make_tuple,            dst: Reg<u8>, desc: Const<u16>);
_asm!(make_tuple_empty,      dst: Reg<u8>);
// _asm!(jump,                  offset: Offset<u24>);
// _asm!(jump_const,            offset: Const<u16>);
// _asm!(jump_loop,             offset: Offset<u24>);
// _asm!(jump_loop_const,       offset: Const<u16>);
// _asm!(jump_if_false,         offset: Offset<u24>);
// _asm!(jump_if_false_const,   offset: Const<u16>);
_asm!(add,                   dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8>);
_asm!(sub,                   dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8>);
_asm!(mul,                   dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8>);
_asm!(div,                   dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8>);
_asm!(rem,                   dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8>);
_asm!(pow,                   dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8>);
_asm!(inv,                   val: Reg<u8>);
_asm!(not,                   val: Reg<u8>);
_asm!(cmp_eq,                dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8>);
_asm!(cmp_ne,                dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8>);
_asm!(cmp_gt,                dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8>);
_asm!(cmp_ge,                dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8>);
_asm!(cmp_lt,                dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8>);
_asm!(cmp_le,                dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8>);
_asm!(cmp_type,              dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8>);
_asm!(contains,              dst: Reg<u8>, lhs: Reg<u8>, rhs: Reg<u8>);
_asm!(is_nil,                dst: Reg<u8>, val: Reg<u8>);
_asm!(call,                  func: Reg<u8>, count: Count<u8>);
_asm!(call0,                 func: Reg<u8>);
_asm!(import,                dst: Reg<u8>, path: Const<u16>);
_asm!(finalize_module);
_asm!(ret,                   val: Reg<u8>);
_asm!(yld,                   val: Reg<u8>);

pub fn jump() -> Op {
  Op::Jump {
    offset: Offset(0u8.into()),
  }
}

pub fn jump_if_false(val: Reg<u8>) -> impl Fn() -> Op {
  move || Op::JumpIfFalse {
    val,
    offset: Offset(0u8.into()),
  }
}

pub fn jump_if_true(val: Reg<u8>) -> impl Fn() -> Op {
  move || Op::JumpIfTrue {
    val,
    offset: Offset(0u8.into()),
  }
}

pub fn jump_loop(offset: JumpOffset) -> Op {
  use JumpOffset::*;
  match offset {
    Short(offset) => Op::JumpLoop { offset },
    Long(offset) => Op::JumpLoopConst { idx: offset },
  }
}
