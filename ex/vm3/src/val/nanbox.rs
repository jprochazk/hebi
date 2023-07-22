use core::fmt::{Debug, Display};
use core::marker::PhantomData;
use core::mem::transmute;

use crate::gc::{Any, Object, Ref};

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
  pub const NIL    : u64 = 0b01111111_11111110_00000000_00000000_00000000_00000000_00000000_00000000;
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
  Nil,
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
  Nil,
  Object,
}

// Constructors
impl Value {
  #[inline(always)]
  pub fn new(v: impl Into<Value>) -> Self {
    v.into()
  }

  /// Construct a `Value` directly from bits.
  ///
  /// # Safety
  /// `bits` must be properly tagged.
  #[inline(always)]
  const unsafe fn from_bits(bits: u64) -> Self {
    Self {
      bits,
      _p: PhantomData,
    }
  }

  #[inline(always)]
  pub fn is<T: ValueType>(self) -> bool {
    T::is(self)
  }

  /// Convert `self` into a `T` if it is currently inhabited by `T`.
  #[inline(always)]
  pub fn cast<T: TryFrom<Value>>(self) -> Option<T> {
    self.try_into().ok()
  }

  /// The unchecked version of [`Value::cast`].
  ///
  /// # Safety
  /// `self.is::<T>()` must be `true`
  #[inline(always)]
  pub unsafe fn coerce<T: TryFrom<Value>>(self) -> T {
    T::try_from(self).unwrap_unchecked()
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
        ty::NIL => Type::Nil,
        ty::OBJECT => Type::Object,
        _ => unsafe { core::hint::unreachable_unchecked() },
      }
    }
  }

  #[inline(always)]
  fn unmask(&self) -> u64 {
    self.bits & mask::VALUE
  }

  #[inline(always)]
  fn type_tag(&self) -> u64 {
    self.bits & mask::TAG
  }
}

pub trait ValueType: private::Sealed {
  fn is(value: Value) -> bool;
}

impl private::Sealed for f64 {}
impl ValueType for f64 {
  #[inline]
  fn is(value: Value) -> bool {
    (value.bits & mask::QNAN) != mask::QNAN
  }
}

impl private::Sealed for i32 {}
impl ValueType for i32 {
  #[inline]
  fn is(value: Value) -> bool {
    value.type_tag() == ty::INT
  }
}

impl private::Sealed for bool {}
impl ValueType for bool {
  #[inline]
  fn is(value: Value) -> bool {
    value.type_tag() == ty::BOOL
  }
}

impl private::Sealed for nil {}
impl ValueType for nil {
  #[inline]
  fn is(value: Value) -> bool {
    value.type_tag() == ty::NIL
  }
}

impl private::Sealed for Any {}
impl ValueType for Any {
  #[inline]
  fn is(value: Value) -> bool {
    value.type_tag() == ty::OBJECT
  }
}

impl<T: Object + 'static> private::Sealed for Ref<T> {}
impl<T: Object + 'static> ValueType for Ref<T> {
  fn is(value: Value) -> bool {
    if value.type_tag() == ty::OBJECT {
      let obj = unsafe { value.coerce::<Any>() };
      obj.is::<T>()
    } else {
      false
    }
  }
}

// Constructors
impl From<f64> for Value {
  #[inline(always)]
  fn from(value: f64) -> Self {
    let bits = value.to_bits();
    if bits & mask::QNAN == mask::QNAN {
      panic!("cannot construct a Value from an f64 which is already a quiet NaN");
    }
    unsafe { Value::from_bits(bits) }
  }
}

impl TryFrom<Value> for f64 {
  type Error = ();

  fn try_from(value: Value) -> Result<f64, Self::Error> {
    if value.is::<f64>() {
      Ok(f64::from_bits(value.bits))
    } else {
      Err(())
    }
  }
}

impl From<i32> for Value {
  #[inline(always)]
  fn from(value: i32) -> Self {
    // We want the bits of `v`, not for it to be reinterpreted as an unsigned int.
    let bits = unsafe { transmute::<i32, u32>(value) } as u64;
    let bits = bits | ty::INT;
    unsafe { Value::from_bits(bits) }
  }
}

impl TryFrom<Value> for i32 {
  type Error = ();

  fn try_from(value: Value) -> Result<i32, Self::Error> {
    if value.is::<i32>() {
      Ok(unsafe { transmute::<u32, i32>(value.unmask() as u32) })
    } else {
      Err(())
    }
  }
}

impl From<bool> for Value {
  #[inline(always)]
  fn from(value: bool) -> Self {
    let bits = value as u64;
    let bits = bits | ty::BOOL;
    unsafe { Value::from_bits(bits) }
  }
}

impl TryFrom<Value> for bool {
  type Error = ();

  fn try_from(value: Value) -> Result<bool, Self::Error> {
    if value.is::<bool>() {
      Ok(value.unmask() != 0)
    } else {
      Err(())
    }
  }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct nil;

impl From<()> for nil {
  #[inline]
  fn from(_: ()) -> Self {
    nil {}
  }
}

impl From<nil> for () {
  #[inline]
  fn from(_: nil) -> Self {}
}

impl From<nil> for Value {
  #[inline]
  fn from(_: nil) -> Self {
    let bits = ty::NIL;
    unsafe { Value::from_bits(bits) }
  }
}

impl TryFrom<Value> for nil {
  type Error = ();

  #[inline]
  fn try_from(value: Value) -> Result<nil, Self::Error> {
    if value.is::<nil>() {
      Ok(nil {})
    } else {
      Err(())
    }
  }
}

impl From<Any> for Value {
  #[inline]
  fn from(value: Any) -> Self {
    let bits = value.addr() as u64;
    let bits = (bits & mask::VALUE) | ty::OBJECT;
    unsafe { Value::from_bits(bits) }
  }
}

impl TryFrom<Value> for Any {
  type Error = ();

  #[inline]
  fn try_from(value: Value) -> Result<Any, Self::Error> {
    if value.is::<Any>() {
      Ok(unsafe { Any::from_addr(value.unmask() as usize) })
    } else {
      Err(())
    }
  }
}

impl<T: Object + 'static> From<Ref<T>> for Value {
  #[inline]
  fn from(value: Ref<T>) -> Self {
    <Value as From<Any>>::from(value.erase())
  }
}

impl<T: Object + 'static> TryFrom<Value> for Ref<T> {
  type Error = ();

  #[inline]
  fn try_from(value: Value) -> Result<Self, Self::Error> {
    <Any as TryFrom<Value>>::try_from(value).and_then(|o| o.cast().ok_or(()))
  }
}

impl Debug for Value {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    if let Some(v) = self.cast::<f64>() {
      Debug::fmt(&v, f)
    } else if let Some(v) = self.cast::<i32>() {
      Debug::fmt(&v, f)
    } else if let Some(v) = self.cast::<bool>() {
      Debug::fmt(&v, f)
    } else if self.is::<nil>() {
      f.debug_tuple("Nil").finish()
    } else {
      let v = unsafe { self.coerce::<Any>() };
      Debug::fmt(&v, f)
    }
  }
}

impl Display for Value {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    if let Some(v) = self.cast::<f64>() {
      Display::fmt(&v, f)
    } else if let Some(v) = self.cast::<i32>() {
      Display::fmt(&v, f)
    } else if let Some(v) = self.cast::<bool>() {
      write!(f, "{v}")
    } else if self.is::<nil>() {
      write!(f, "nil")
    } else {
      Display::fmt(&unsafe { self.coerce::<Any>() }, f)
    }
  }
}

mod private {
  pub trait Sealed {}
}

#[cfg(test)]
mod tests {
  use core::fmt::Display;

  use super::*;
  use crate::gc::{Gc, Object};

  #[test]
  fn to_int() {
    let v = Value::new(0i32);
    assert!(v.is::<i32>());
    let v = v.cast::<i32>().unwrap();
    assert_eq!(v, 0i32);
  }

  #[test]
  fn to_float() {
    let v = Value::new(0.1f64);
    assert!(v.is::<f64>());
    let v = v.cast::<f64>().unwrap();
    assert_eq!(v, 0.1f64);
  }

  #[test]
  fn to_bool() {
    let v = Value::new(true);
    assert!(v.is::<bool>());
    let v = v.cast::<bool>().unwrap();
    assert!(v);
  }

  #[test]
  fn to_nil() {
    let v = Value::new(nil);
    assert!(v.is::<nil>());
    v.cast::<nil>().unwrap();
  }

  #[test]
  fn to_obj() {
    #[derive(Debug)]
    struct Foo {
      n: i32,
    }
    impl Object for Foo {}
    impl Display for Foo {
      fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "T")
      }
    }

    let gc = Gc::default();
    let v = gc.try_alloc(Foo { n: i16::MAX as i32 }).unwrap().erase();
    let v = Value::new(v);
    assert!(v.is::<Any>());
    assert!(v.is::<Ref<Foo>>());
    {
      let v = v.cast::<Any>().unwrap();
      let v = v.cast::<Foo>().unwrap();
      assert_eq!(v.n, i16::MAX as i32);
    }
    {
      let v = v.cast::<Ref<Foo>>().unwrap();
      assert_eq!(v.n, i16::MAX as i32);
    }
  }
}
