//! Mu's core value type.
//!
//! Mu values are 64 bits, and utilize NaN boxing (also known as NaN tagging)
//! for packing both the type tag bits and value bits into a single 64-bit
//! package.
//!
//! A 64-bit floating-point number is stored by separating the exponent and
//! mantissa bits. The mantissa tells you the actual numerical value, and the
//! exponent tells you where the decimal point goes.
//!
//! Floating-point numbers have special values known as NaNs (Not a Number).
//! NaNs are used to represent invalid numbers, such as those produced when you
//! divide by zero. There are two main kinds of NaNs:
//!
//! - Signalling NaNs
//! - Quiet NaNs
//!
//! Doing arithmetic with a signalling NaN will result in the CPU aborting the
//! process, but doing so with a quiet NaN will simply produce another quiet
//! NaN.
//!
//! A quiet NaN looks like this:
//!
//! ```text,ignore
//!    ┌────────────────────────────────────────────────────────────────────────┐
//!    │01111111_11111100_00000000_00000000_00000000_00000000_00000000_000000000│
//!    └▲▲──────────▲▲▲───▲────────────────────────────────────────────────────▲┘
//!     │└┬─────────┘││   └──────┬─────────────────────────────────────────────┘
//!     │ └Exponent  ││          └Mantissa
//!     │            │└Intel FP Indef.
//!     └Sign bit    └Quiet bit
//! ```
//!
//! In the above representation, all of the zeroed bits may contain arbitrary
//! values.
//!
//! The second peculiarity that makes NaN boxing work is that no modern 64-bit
//! operating system actually uses the full 64-bit address space. Pointers only
//! ever use the low 48 bits, and the high 16 bits are zeroed out.
//!
//! If we overlap a 48-bit pointer pointing to the end of a 48-bit address range
//! with a quiet NaN, we get something that looks like this:
//!
//! ```text,ignore
//!    ┌───────────────────────────────────────────────────────────────────────┐
//!    │01111111_11111100_11111111_11111111_11111111_11111111_11111111_11111111│
//!    └───────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! What this means is that we have 3 bits which are completely unused in both
//! pointers and quiet NaNs.
//!
//! NaN boxing is a technique that takes advantage of this "hole" in the
//! possible bit patterns of pointers and quiet NaNs by storing the type of the
//! value in the free bits.
//!
//! Mu values have 5 possible types:
//! - Float
//! - Int
//! - None
//! - Bool
//! - Object
//!
//! ### Float
//!
//! If a value is not a quiet NaN, then it is a float. This means that floats do
//! not require any conversions before usage, only a type check.
//!
//! ```text,ignore
//!    Tag = Not a quiet NaN
//!   ┌┴───────────┐
//! ┌─▼────────────▼────────────────────────────────────────────────────────┐
//! │........_........_........_........_........_........_........_........│
//! └───────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Int
//!
//! A quiet NaN with all free bits zeroed represents an integer. This makes
//! integers slightly more expensive to use, as the high 16 bits (except the
//! sign bit) need to be cleared after a type check. It also means that integers
//! may only hold up to 48 bits.
//!
//! ```text,ignore
//!   Tag = 000
//!  ┌┴─────────────┬┐
//! ┌▼──────────────▼▼──────────────────────────────────────────────────────┐
//! │s1111111_11111100_00000000_00000000_00000000_00000000_00000000_00000000│
//! └───────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Bool
//!
//! A quiet NaN with the free bit pattern `001` represents a boolean.
//!
//! ```text,ignore
//!   Tag = 001
//!  ┌┴─────────────┬┐
//! ┌▼──────────────▼▼──────────────────────────────────────────────────────┐
//! │01111111_11111101_00000000_00000000_00000000_00000000_00000000_00000000│
//! └───────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### None
//!
//! A quiet NaN with the free bit pattern `010` represents a `None` value.
//!
//! ```text,ignore
//!   Tag = 010
//!  ┌┴─────────────┬┐                                                Value┐
//! ┌▼──────────────▼▼─────────────────────────────────────────────────────▼┐
//! │01111111_11111110_00000000_00000000_00000000_00000000_00000000_0000000X│
//! └───────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Object
//!
//! A quiet NaN with the free bit pattern `100` represents an `Object` pointer.
//!
//! ```text,ignore
//!   Tag = 011
//!  ┌┴─────────────┬┐
//! ┌▼──────────────▼▼──────────────────────────────────────────────────────┐
//! │01111111_11111111_00000000_00000000_00000000_00000000_00000000_00000000│
//! └───────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Implementation inspired by <https://github.com/Marwes/nanbox> and <http://craftinginterpreters.com/optimization.html>.
//! See the last link for more info.
#![allow(clippy::unusual_byte_groupings)]

// TODO: object representation
// maybe just a trait?

pub mod object;

use std::marker::PhantomData;
use std::rc::Rc;

use object::Ptr;

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
  Object(Ptr),
}

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
    let bits = v as u32 as u64;
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

  pub fn object(v: Ptr) -> Self {
    let ptr = Rc::into_raw(v.0);
    println!("{ptr:#p}");
    let ptr = ptr as u64;
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

  pub fn to_object(self) -> Option<Ptr> {
    if !self.is_object() {
      return None;
    }
    let ptr = self.value() as *const object::Temp;
    println!("{ptr:#p}");
    std::mem::forget(self);
    Some(Ptr(unsafe { Rc::from_raw(ptr) }))
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
      let ptr = self.value() as *const object::Temp;
      debug_assert!(!ptr.is_null());
      println!("{ptr:#p}");
      // Then reconstruct the pointer
      let ptr = unsafe { Rc::from_raw(ptr) };
      let value = Value::object(Ptr(ptr.clone()));
      let _ = std::mem::ManuallyDrop::new(ptr);
      value
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
      let ptr = self.value() as *const object::Temp;
      unsafe { Rc::decrement_strong_count(ptr) }
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
