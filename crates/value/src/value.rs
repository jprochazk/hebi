#![allow(clippy::unusual_byte_groupings)]

use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use crate::object::Object;
use crate::ptr::Ptr;

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
  Object(Ptr<Object>),
}

/// Mu's core `Value` type.
///
/// See the [index][`crate`] for more about the different value types and their
/// encodings.
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
/// require executing bytecode via the VM. If you need value equality, you have
/// to go through the VM.
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
    let bits = unsafe { std::mem::transmute::<_, u32>(v) } as u64;
    let bits = bits | ty::INT;
    Self::new(bits)
  }

  // 0b000000_00000000_01111111_00111001_00101000_00000000_00001101_00100000

  pub fn bool(v: bool) -> Self {
    let bits = v as u64;
    let bits = bits | ty::BOOL;
    Self::new(bits)
  }

  pub fn none() -> Self {
    let bits = ty::NONE;
    Self::new(bits)
  }

  pub fn object(v: Ptr<Object>) -> Self {
    let ptr = Ptr::into_addr(v) as u64;
    let bits = (ptr & mask::VALUE) | ty::OBJECT;
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

// Owned conversions
impl Value {
  pub fn to_float(self) -> Option<f64> {
    if !self.is_float() {
      return None;
    }
    Some(f64::from_bits(self.bits))
  }

  pub fn to_int(self) -> Option<i32> {
    if !self.is_int() {
      return None;
    }
    Some(self.value() as u32 as i32)
  }

  pub fn to_bool(self) -> Option<bool> {
    if !self.is_bool() {
      return None;
    }
    Some(self.value() == 1)
  }

  pub fn to_none(self) -> Option<()> {
    if !self.is_none() {
      return None;
    }
    Some(())
  }

  pub fn to_object(self) -> Option<Ptr<Object>> {
    if !self.is_object() {
      return None;
    }
    let ptr = unsafe { Ptr::from_addr(self.value() as usize) };
    std::mem::forget(self);
    Some(ptr)
  }
}

impl PartialEq<Value> for Value {
  fn eq(&self, other: &Value) -> bool {
    if self.is_float() && other.is_float() {
      f64::from_bits(self.bits) == f64::from_bits(other.bits)
    } else {
      self.bits == other.bits
    }
  }
}
// Note: NaNs are not reflexive, but we close our eyes,
// and pray that this doesn't break things too badly.
// We do this to be able to store `Value` as a key in a `HashMap`.
impl Eq for Value {}

impl Hash for Value {
  fn hash<H: Hasher>(&self, state: &mut H) {
    let value = if self.is_float() && f64::from_bits(self.bits).is_nan() {
      // all NaNs have the same hash
      mask::QNAN
    } else {
      self.bits
    };
    value.hash(state)
  }
}

impl Value {
  /// Checks if `self` and `other` hold the same value.
  ///
  /// Note that in case both values are objects, this only checks for reference
  /// equality. This is because classes in user scripts may overload the
  /// equality operation, in which case the VM must call a method in order to
  /// determine whether two objects are equal according to the user's
  /// definition.
  pub fn is_eq(&self, other: &Value) -> bool {
    if self.is_float() && other.is_float() {
      f64::from_bits(self.bits) == f64::from_bits(other.bits)
    } else {
      self.type_tag() == other.type_tag() && self.bits == other.bits
    }
  }
}

impl Clone for Value {
  fn clone(&self) -> Self {
    if self.is_object() {
      let addr = self.value() as usize;
      unsafe { Ptr::increment_strong_count(addr) }
      let ptr = unsafe { Ptr::from_addr(addr) };
      Value::object(ptr)
    } else {
      // SAFETY: this is not an object, so we don't need to increment the reference
      // count.
      Self {
        bits: self.bits,
        _p: self._p,
      }
    }
  }
}

impl Drop for Value {
  fn drop(&mut self) {
    if self.is_object() {
      // Decrement the reference count of `self`
      unsafe { Ptr::decrement_strong_count(self.value() as usize) }
    }
  }
}

impl std::fmt::Display for Value {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let v = self.clone();
    if v.is_float() {
      write!(f, "{}", v.to_float().unwrap())?;
    } else if v.is_int() {
      write!(f, "{}", v.to_int().unwrap())?;
    } else if v.is_bool() {
      write!(f, "{}", v.to_bool().unwrap())?;
    } else if v.is_none() {
      write!(f, "none")?;
    } else if v.is_object() {
      write!(f, "{}", v.to_object().unwrap())?;
    } else {
      unreachable!("invalid type");
    }

    Ok(())
  }
}

impl std::fmt::Debug for Value {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let v = self.clone();
    let mut s = f.debug_struct("Value");
    if v.is_float() {
      s.field("type", &"float");
      s.field("value", &v.to_float());
    } else if v.is_int() {
      s.field("type", &"int");
      s.field("value", &v.to_int());
    } else if v.is_bool() {
      s.field("type", &"bool");
      s.field("value", &v.to_bool());
    } else if v.is_none() {
      s.field("type", &"none");
      s.field("value", &"<none>");
    } else if v.is_object() {
      s.field("type", &"object");
      s.field("value", &v.to_object());
    } else {
      unreachable!("invalid type");
    }
    s.finish()
  }
}

#[cfg(test)]
mod tests;
