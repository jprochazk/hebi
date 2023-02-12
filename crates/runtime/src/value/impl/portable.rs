use super::*;

// TODO: make this indistinguishable from nanbox value

#[derive(Clone)]
pub enum Value {
  Float(f64),
  Int(i32),
  Bool(bool),
  None,
  Object(Ptr<object::Object>),
}

impl Value {
  pub fn float(v: f64) -> Self {
    Self::Float(v)
  }

  pub fn int(v: i32) -> Self {
    Self::Int(v)
  }

  // 0b000000_00000000_01111111_00111001_00101000_00000000_00001101_00100000

  pub fn bool(v: bool) -> Self {
    Self::Bool(v)
  }

  pub fn none() -> Self {
    Self::None
  }

  pub fn object(v: Ptr<object::Object>) -> Self {
    Self::Object(v)
  }
}

// Type checks
impl Value {
  #[inline]
  pub fn is_float(&self) -> bool {
    matches!(self, Self::Float(..))
  }

  #[inline]
  pub fn is_int(&self) -> bool {
    matches!(self, Self::Int(..))
  }

  #[inline]
  pub fn is_bool(&self) -> bool {
    matches!(self, Self::Bool(..))
  }

  #[inline]
  pub fn is_none(&self) -> bool {
    matches!(self, Self::None)
  }

  #[inline]
  pub fn is_object(&self) -> bool {
    matches!(self, Self::Object(..))
  }

  /// This is `pub(crate)` so that users may not break the invariant that
  /// `value::object::dict::Key::String` is always a string by mutating the
  /// object behind the pointer using `borrow_mut`.
  ///
  /// It's not necessary because `Value` has impls for `as_<repr>` where
  /// `<repr>` is any of the possible object representations.
  pub(crate) fn into_object(self) -> Option<Ptr<object::Object>> {
    if !self.is_object() {
      return None;
    }

    match self {
      Value::Object(v) => Some(v),
      _ => unreachable!(),
    }
  }

  pub(crate) fn as_object(&self) -> Option<Ref<'_, object::Object>> {
    match self {
      Value::Object(v) => Some(v.borrow()),
      _ => None,
    }
  }

  /// This is `pub(crate)` so that users may not break the invariant that
  /// `value::object::dict::Key::String` is always a string by mutating the
  /// object behind the pointer using `borrow_mut`.
  ///
  /// It's not necessary because `Value` has impls for `as_<repr>` where
  /// `<repr>` is any of the possible object representations.
  pub(crate) fn as_object_mut(&mut self) -> Option<RefMut<'_, object::Object>> {
    match self {
      Value::Object(v) => Some(v.borrow_mut()),
      _ => None,
    }
  }
}

// Owned conversions
impl Value {
  pub fn as_float(&self) -> Option<f64> {
    match self {
      Value::Float(v) => Some(*v),
      _ => None,
    }
  }

  pub fn to_float(self) -> Option<f64> {
    self.as_float()
  }

  pub fn as_int(&self) -> Option<i32> {
    match self {
      Value::Int(v) => Some(*v),
      _ => None,
    }
  }

  pub fn to_int(self) -> Option<i32> {
    self.as_int()
  }

  pub fn as_bool(&self) -> Option<bool> {
    match self {
      Value::Bool(v) => Some(*v),
      _ => None,
    }
  }

  pub fn to_bool(self) -> Option<bool> {
    self.as_bool()
  }

  pub fn as_none(&self) -> Option<()> {
    match self {
      Value::None => Some(()),
      _ => None,
    }
  }

  pub fn to_none(self) -> Option<()> {
    self.as_none()
  }
}

impl PartialEq<Value> for Value {
  fn eq(&self, other: &Value) -> bool {
    match (self, other) {
      (Value::Float(a), Value::Float(b)) => a == b,
      (Value::Int(a), Value::Int(b)) => a == b,
      (Value::Bool(a), Value::Bool(b)) => a == b,
      (Value::None, Value::None) => true,
      (Value::Object(a), Value::Object(b)) => a == b,
      (_, _) => false,
    }
  }
}
// Note: NaNs are not reflexive, but we close our eyes,
// and pray that this doesn't break things too badly.
// We do this to be able to store `Value` as a key in a `HashMap`.
impl Eq for Value {}

const QNAN: u64 = 0b01111111_11111100_00000000_00000000_00000000_00000000_00000000_00000000;

impl Hash for Value {
  fn hash<H: Hasher>(&self, state: &mut H) {
    match self {
      Value::Float(v) => {
        if v.is_nan() {
          QNAN.hash(state)
        } else {
          v.to_bits().hash(state)
        }
      }
      Value::Int(v) => v.hash(state),
      Value::Bool(v) => v.hash(state),
      Value::None => 0.hash(state),
      Value::Object(v) => v.borrow().hash(state),
    }
  }
}
