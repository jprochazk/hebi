#![allow(non_camel_case_types)]

// TODO: describe remaining opcodes
// TODO: annotate which opcodes use extra data
// TODO: `is` and `in`

/// An opcode represents a basic operation that the VM may perform.
///
/// Each opcode may store a maximum of 24 bits as operands. For opcodes
/// that require more than 24 bits of data, the additional data may be
/// stored in the bytecode array as opaque bytes following the main opcode.
///
/// Here's an example of the `GetField` opcode with an extra field for storing a
/// pointer to an inline cache:
/// ```text
///   index opcode   fields
///   0     GetField (tag:u8), slot:u16, padding:u8
///   1     Data     ic_ptr:u32
///   2     Data           :u32
/// ```
/// The `Data` opcodes are not real opcodes, as the `Opcode` enum does not
/// contain a variant named `Data`. The byte normally used to store the enum
/// discriminant is re-used to store an additional byte of data. Because the
/// `data` is stored sequentially in the bytecode, the VM may use a `transmute`
/// to reinterpret this data as a pointer to the instruction's inline cache
/// with no overhead.
#[repr(u8)]
pub enum Opcode {
  /// Push a value from the current function's constants pool onto the stack.
  ///
  /// ```text
  ///   op       idx stack pool captures globals
  ///   GetConst 0   [ ]   [a]  [ ]      {}
  ///   _        _   [a]   [a]  [ ]      {}
  /// ```
  Op_GetConst {
    slot: u16,
  },
  /// Copy a value at stack `slot` to the top of the stack.
  ///
  /// ```text
  ///   op       idx stack  pool captures globals
  ///   GetLocal 0   [a]    [ ]  [ ]      {}
  ///   _        _   [a, a] [ ]  [ ]      {}
  /// ```
  Op_GetLocal {
    slot: u16,
  },
  /// Copy a value from the top of the stack to stack `slot`.
  ///
  /// ```text
  ///   op       idx stack  pool captures globals
  ///   SetLocal 0   [a, b] [ ]  [ ]      {}
  ///   _        _   [b, b] [ ]  [ ]      {}
  /// ```
  Op_SetLocal {
    slot: u16,
  },
  /// Copy a value from a `slot` in the current function's captures to the top
  /// of the stack.
  ///
  /// ```text
  ///   op         idx stack pool captures globals
  ///   GetCapture 0   [ ]   [ ]  [a]      {}
  ///   _          _   [a]   [ ]  [a]      {}
  /// ```
  Op_GetCapture {
    slot: u16,
  },
  /// Copy a value from the top of the stack to a `slot` in the current
  /// function's captures.
  ///
  /// ```text
  ///   op         idx stack pool captures globals
  ///   GetCapture 0   [a]   [ ]  [ ]      {}
  ///   _          _   [a]   [ ]  [a]      {}
  /// ```
  Op_SetCapture {
    slot: u16,
  },
  /// Copy a value from a global `slot` to the top of the stack.
  ///
  /// `slot` is an index into the constant pool
  /// where the name of the global is stored.
  ///
  /// ```text
  ///   op        idx stack pool captures globals
  ///   GetGlobal 0   [ ]   [a]  [ ]      { a: v }
  ///   _         _   [v]   [ ]  [ ]      { a: v }
  /// ```
  Op_GetGlobal {
    slot: u16,
  },
  /// Copy a value from the top of the stack to a global `slot`.
  ///
  /// `slot` is an index into the constant pool
  /// where the name of the global is stored.
  ///
  /// ```text
  ///   op        idx stack pool captures globals
  ///   SetGlobal 0   [v]   [a]  [ ]      {}
  ///   _         _   [v]   [ ]  [ ]      { a: v }
  /// ```
  Op_SetGlobal {
    slot: u16,
  },
  /// Copy a value from an object's field to the top of the stack.
  ///
  /// ```text
  ///   op       idx stack            pool captures globals
  ///   GetField 0   [{ a: v }, a]    [ ]  [ ]      {}
  ///   _        _   [{ a: v }, a, v] [ ]  [ ]      {}
  /// ```
  Op_GetField,
  /// Copy a value from the top of the stack to an object's field.
  ///
  /// ```text
  ///   op         idx stack            pool captures globals
  ///   GetGlobal  0   [{ a: _ }, a, v] [ ]  [ ]      {}
  ///   _          _   [{ a: v }, a, v] [ ]  [ ]      {}
  /// ```
  Op_SetField,

  Op_CreateArray {
    descriptor: u16,
  },
  Op_CreateObject {
    descriptor: u16,
  },
  Op_CreateClosure {
    descriptor: u16,
  },
  Op_CreateClass {
    descriptor: u16,
  },

  Op_Call,

  Op_Push,
  Op_Pop {
    n: u16,
  },
  Op_Copy {
    n: u16,
  },
  Op_Jump {
    offset: u16,
  },
  Op_JumpIfFalse {
    offset: u16,
  },
  Op_Yield,
  Op_Return,

  Op_Add,
  Op_Subtract,
  Op_Multiply,
  Op_Divide,
  Op_Remainder,
  Op_Power,
  Op_Negate,
  Op_Not,
  Op_Equal,
  Op_CompareLess,
  Op_CompareMore,
}
static_assertions::assert_eq_size!(Opcode, u32);

// /// A 24-bit unsigned integer.
// ///
// /// This is only useful for storage,
// /// it must be converted to/from a u32 before being used.
// pub struct u24([u8; 3]);
// static_assertions::assert_eq_size!(u24, [u8; 3]);
//
// impl From<u24> for u32 {
//   fn from(v: u24) -> Self {
//     let [a, b, c] = v.0;
//     ((a as u32) << 16) + ((b as u32) << 8) + (c as u32)
//   }
// }
//
// impl TryFrom<u32> for u24 {
//   type Error = ();
//   fn try_from(v: u32) -> Result<Self, Self::Error> {
//     if v > 0x00FFFFFF {
//       Err(())
//     } else {
//       Ok(u24([
//         (v & 0x00ff0000 >> 16) as u8,
//         (v & 0x0000ff00 >> 8) as u8,
//         (v & 0x000000ff) as u8,
//       ]))
//     }
//   }
// }
