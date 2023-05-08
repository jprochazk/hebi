use std::marker::PhantomData;
use std::mem;

use crate::object::ptr::{Any, Ptr};
use crate::object::Object;

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
  Object(Ptr<Any>),
}

/// Hebi's core `Value` type.
///
/// See the [index][`crate`] for more about the different value types and
/// their encodings.
///
/// ### Equality
///
/// Two `Value`s are considered equal if:
/// - they are both `NaN` floats, or
/// - they are both floats with an absolute value of `0`, or
/// - their underlying bit values are the same
///
/// Objects are compared by reference, not by value. This is because an object
/// may override the equality operation with arbitrary code which may even
/// require executing bytecode via the VM. If you need value equality, you
/// have to go through the VM.
pub struct Value {
  bits: u64,
  _p: PhantomData<PhantomValue>,
}

// Constructors
impl Value {
  fn new(bits: u64) -> Self {
    Self {
      bits,
      _p: PhantomData,
    }
  }

  pub fn float(v: f64) -> Self {
    let bits = v.to_bits();
    if bits & mask::QNAN == mask::QNAN {
      panic!("cannot construct a Value from an f64 which is already a quiet NaN");
    }
    Self::new(bits)
  }

  pub fn int(v: i32) -> Self {
    // We want the bits of `v`, not for it to be reinterpreted as an unsigned int.
    let bits = unsafe { std::mem::transmute::<i32, u32>(v) } as u64;
    let bits = bits | ty::INT;
    Self::new(bits)
  }

  pub fn bool(v: bool) -> Self {
    let bits = v as u64;
    let bits = bits | ty::BOOL;
    Self::new(bits)
  }

  pub fn none() -> Self {
    let bits = ty::NONE;
    Self::new(bits)
  }

  pub fn object<T: Object>(ptr: Ptr<T>) -> Self {
    let bits = ptr.into_addr() as u64;
    let bits = (bits & mask::VALUE) | ty::OBJECT;
    Self::new(bits)
  }
}

// Type checks
impl Value {
  #[inline]
  fn value(&self) -> u64 {
    self.bits & mask::VALUE
  }

  #[inline]
  fn type_tag(&self) -> u64 {
    self.bits & mask::TAG
  }

  #[inline]
  pub fn is_float(&self) -> bool {
    (self.bits & mask::QNAN) != mask::QNAN
  }

  #[inline]
  pub fn is_int(&self) -> bool {
    self.type_tag() == ty::INT
  }

  #[inline]
  pub fn is_bool(&self) -> bool {
    self.type_tag() == ty::BOOL
  }

  #[inline]
  pub fn is_none(&self) -> bool {
    self.type_tag() == ty::NONE
  }

  #[inline]
  pub fn is_object(&self) -> bool {
    self.type_tag() == ty::OBJECT
  }
}

impl Clone for Value {
  fn clone(&self) -> Self {
    if self.is_object() {
      unsafe { Ptr::<Any>::incref_addr(self.value() as usize) }
    }

    Self {
      bits: self.bits,
      _p: self._p,
    }
  }
}

impl Drop for Value {
  fn drop(&mut self) {
    if self.is_object() {
      unsafe {
        Ptr::<Any>::from_addr(self.value() as usize);
      }
    }
  }
}

// Owned conversions
impl Value {
  pub fn to_float(self) -> Option<f64> {
    if !self.is_float() {
      return None;
    }
    Some(unsafe { self.to_float_unchecked() })
  }

  /// # Safety
  /// - `self.is_float()` must be `true`
  pub unsafe fn to_float_unchecked(self) -> f64 {
    debug_assert!(self.is_float(), "value is not a float");
    f64::from_bits(self.bits)
  }

  pub fn to_int(self) -> Option<i32> {
    if !self.is_int() {
      return None;
    }
    Some(unsafe { self.to_int_unchecked() })
  }

  /// # Safety
  /// - `self.is_int()` must be `true`
  pub unsafe fn to_int_unchecked(self) -> i32 {
    debug_assert!(self.is_int(), "value is not an int");
    self.value() as u32 as i32
  }

  pub fn to_bool(self) -> Option<bool> {
    if !self.is_bool() {
      return None;
    }
    Some(self.value() == 1)
  }

  /// # Safety
  /// - `self.is_bool()` must be `true`
  pub unsafe fn to_bool_unchecked(self) -> bool {
    debug_assert!(self.is_bool(), "value is not a bool");
    self.value() != 0
  }

  pub fn to_none(self) -> Option<()> {
    if !self.is_none() {
      return None;
    }
    Some(())
  }

  /// # Safety
  /// - `self.is_none()` must be `true`
  #[allow(clippy::unused_unit)]
  pub unsafe fn to_none_unchecked(self) -> () {
    debug_assert!(self.is_none(), "value is not none");
    ()
  }

  pub fn to_object(self) -> Option<Ptr<Any>> {
    if !self.is_object() {
      return None;
    }
    Some(unsafe { self.to_object_unchecked() })
  }

  /// # Safety
  /// - `self.is_object()` must be `true`
  pub unsafe fn to_object_unchecked(self) -> Ptr<Any> {
    debug_assert!(self.is_object(), "value is not an object");
    let ptr = unsafe { Ptr::from_addr(self.value() as usize) };
    mem::forget(self);
    ptr
  }
}
