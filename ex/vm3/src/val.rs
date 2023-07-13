#![allow(clippy::wrong_self_convention)]

use std::fmt::{Debug, Display};
use std::marker::PhantomData;

use crate::gc::Any;

mod mask {
  //! Generic mask bits

  /// Used to determine if a value is a quiet NAN.
  pub const QNAN: u64 = 0b01111111_11111100_00000000_00000000_00000000_00000000_00000000_00000000;
  /// Used to check the type tag.
  pub const TAG: u64 = 0b11111111_11111111_00000000_00000000_00000000_00000000_00000000_00000000;
  /// Used to mask the 48 value bits.
  pub const VALUE: u64 = 0b00000000_00000000_11111111_11111111_11111111_11111111_11111111_11111111;
}
#[rustfmt::skip]
mod ty {
  //                          Tag
  //                         ┌┴─────────────┬┐
  //                         ▼              ▼▼
  pub const INT    : u64 = 0b01111111_11111100_00000000_00000000_00000000_00000000_00000000_00000000;
  pub const BOOL   : u64 = 0b01111111_11111101_00000000_00000000_00000000_00000000_00000000_00000000;
  pub const NONE   : u64 = 0b01111111_11111110_00000000_00000000_00000000_00000000_00000000_00000000;
  pub const OBJECT : u64 = 0b01111111_11111111_00000000_00000000_00000000_00000000_00000000_00000000;
}

/// A value may contain any of these types, and it's important to let the
/// compiler know about that due to the drop check.
///
/// https://doc.rust-lang.org/nomicon/dropck.html
#[allow(dead_code)]
enum PhantomValue {
  Float(f64),
  Int(i32),
  Bool(bool),
  None,
  Object(Any),
}

/// Hebi's core `Value` type.
#[derive(Clone, Copy)]
pub struct Value {
  bits: u64,
  _p: PhantomData<PhantomValue>,
}

#[derive(Clone, Copy)]
pub enum Type {
  Float,
  Int,
  Bool,
  None,
  Object,
}

// Constructors
impl Value {
  #[inline(always)]
  const fn new(bits: u64) -> Self {
    Self {
      bits,
      _p: PhantomData,
    }
  }

  /// Returns the type of this value.
  ///
  /// Note that using this with a `match` is much slower than
  /// a chain of `if v.is_<type>` statements.
  pub fn ty(&self) -> Type {
    let bits = self.bits;

    if (bits & mask::QNAN) != mask::QNAN {
      Type::Float
    } else {
      let tag = bits & mask::TAG;

      match tag {
        ty::INT => Type::Int,
        ty::BOOL => Type::Bool,
        ty::NONE => Type::None,
        ty::OBJECT => Type::Object,
        _ => unsafe { core::hint::unreachable_unchecked() },
      }
    }
  }

  #[inline(always)]
  pub fn float(v: f64) -> Self {
    let bits = v.to_bits();
    if bits & mask::QNAN == mask::QNAN {
      panic!("cannot construct a Value from an f64 which is already a quiet NaN");
    }
    Self::new(bits)
  }

  #[inline(always)]
  pub fn int(v: i32) -> Self {
    // We want the bits of `v`, not for it to be reinterpreted as an unsigned int.
    let bits = unsafe { std::mem::transmute::<i32, u32>(v) } as u64;
    let bits = bits | ty::INT;
    Self::new(bits)
  }

  #[inline(always)]
  pub fn bool(v: bool) -> Self {
    let bits = v as u64;
    let bits = bits | ty::BOOL;
    Self::new(bits)
  }

  #[inline(always)]
  pub const fn none() -> Self {
    let bits = ty::NONE;
    Self::new(bits)
  }

  #[inline(always)]
  pub fn object(ptr: Any) -> Self {
    let bits = ptr.addr() as u64;
    let bits = (bits & mask::VALUE) | ty::OBJECT;
    Self::new(bits)
  }
}

// Type checks
impl Value {
  #[inline(always)]
  fn value(&self) -> u64 {
    self.bits & mask::VALUE
  }

  #[inline(always)]
  fn type_tag(&self) -> u64 {
    self.bits & mask::TAG
  }

  #[inline(always)]
  pub fn is_float(&self) -> bool {
    (self.bits & mask::QNAN) != mask::QNAN
  }

  #[inline(always)]
  pub fn is_int(&self) -> bool {
    self.type_tag() == ty::INT
  }

  #[inline(always)]
  pub fn is_bool(&self) -> bool {
    self.type_tag() == ty::BOOL
  }

  #[inline(always)]
  pub fn is_none(&self) -> bool {
    self.type_tag() == ty::NONE
  }

  #[inline(always)]
  pub fn is_object(&self) -> bool {
    self.type_tag() == ty::OBJECT
  }
}

impl Value {
  #[inline(always)]
  pub fn to_float(self) -> Option<f64> {
    if self.is_float() {
      Some(unsafe { self.to_float_unchecked() })
    } else {
      None
    }
  }

  /// # Safety
  /// `self.is_float()` must be `true`
  #[inline(always)]
  pub unsafe fn to_float_unchecked(self) -> f64 {
    debug_assert!(self.is_float());
    f64::from_bits(self.bits)
  }

  #[inline(always)]
  pub fn to_int(self) -> Option<i32> {
    if self.is_int() {
      Some(unsafe { self.to_int_unchecked() })
    } else {
      None
    }
  }

  /// # Safety
  /// `self.is_int()` must be `true`
  #[inline(always)]
  pub unsafe fn to_int_unchecked(self) -> i32 {
    debug_assert!(self.is_int());
    self.value() as u32 as i32
  }

  #[inline(always)]
  pub fn to_bool(self) -> Option<bool> {
    if self.is_bool() {
      Some(unsafe { self.to_bool_unchecked() })
    } else {
      None
    }
  }

  /// # Safety
  /// `self.is_bool()` must be `true`
  #[allow(clippy::transmute_int_to_bool)]
  #[inline(always)]
  pub unsafe fn to_bool_unchecked(self) -> bool {
    debug_assert!(self.is_bool());
    unsafe { ::core::mem::transmute(self.value() as u8) }
  }

  #[inline(always)]
  pub fn to_none(self) -> Option<()> {
    if self.is_none() {
      Some(())
    } else {
      None
    }
  }

  /// # Safety
  /// `self.is_none()` must be `true`
  #[allow(clippy::unused_unit)]
  #[inline(always)]
  pub unsafe fn to_none_unchecked(self) -> () {
    debug_assert!(self.is_none());
    ()
  }

  #[inline(always)]
  pub fn to_object(self) -> Option<Any> {
    if self.is_object() {
      Some(unsafe { self.to_object_unchecked() })
    } else {
      None
    }
  }

  /// # Safety
  /// `self.is_object()` must be `true`
  #[inline(always)]
  pub unsafe fn to_object_unchecked(self) -> Any {
    debug_assert!(self.is_object());
    unsafe { Any::from_addr(self.value() as usize) }
  }
}

impl Debug for Value {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if self.is_float() {
      f.debug_tuple("Float")
        .field(&unsafe { self.to_float_unchecked() })
        .finish()
    } else if self.is_int() {
      f.debug_tuple("Int")
        .field(&unsafe { self.to_int_unchecked() })
        .finish()
    } else if self.is_bool() {
      f.debug_tuple("Bool")
        .field(&unsafe { self.to_bool_unchecked() })
        .finish()
    } else if self.is_none() {
      f.debug_tuple("None").finish()
    } else {
      Debug::fmt(&unsafe { self.to_object_unchecked() }, f)
    }
  }
}

impl Display for Value {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    if self.is_float() {
      write!(f, "{}", unsafe { self.to_float_unchecked() })
    } else if self.is_int() {
      write!(f, "{}", unsafe { self.to_int_unchecked() })
    } else if self.is_bool() {
      write!(f, "{}", unsafe { self.to_bool_unchecked() })
    } else if self.is_none() {
      write!(f, "()")
    } else {
      Display::fmt(&unsafe { self.to_object_unchecked() }, f)
    }
  }
}

#[cfg(test)]
mod tests {
  use std::fmt::Display;

  use super::*;
  use crate::gc::{Gc, Object};

  #[test]
  fn to_int() {
    let v = 0i32;
    let v = Value::int(v);
    let v = v.to_int().unwrap();
    assert_eq!(v, 0i32);
  }

  #[test]
  fn to_float() {
    let v = 0.1f64;
    let v = Value::float(v);
    let v = v.to_float().unwrap();
    assert_eq!(v, 0.1f64);
  }

  #[test]
  fn to_bool() {
    let v = true;
    let v = Value::bool(v);
    let v = v.to_bool().unwrap();
    assert!(v);
  }

  #[test]
  fn to_none() {
    let v = Value::none();
    v.to_none().unwrap();
  }

  #[test]
  fn to_obj() {
    #[derive(Debug)]
    struct T {
      n: i32,
    }
    impl Object for T {}
    impl Display for T {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "T")
      }
    }

    let gc = Gc::default();
    let v = gc.alloc(T { n: i16::MAX as i32 }).erase();
    let v = Value::object(v);
    let v = v.to_object().unwrap();
    let v = v.cast::<T>().unwrap();
    assert_eq!(v.n, i16::MAX as i32);
  }
}
