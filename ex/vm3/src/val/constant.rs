use crate::gc::Ref;
use crate::obj::string::Str;

pub enum Constant {
  Float(NFloat),
  Offset(u64),
  Str(Ref<Str>),
}

impl From<NFloat> for Constant {
  fn from(value: NFloat) -> Self {
    Self::Float(value)
  }
}

impl From<u64> for Constant {
  fn from(value: u64) -> Self {
    Self::Offset(value)
  }
}

impl From<Ref<Str>> for Constant {
  fn from(value: Ref<Str>) -> Self {
    Self::Str(value)
  }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct NFloat(u64);

impl NFloat {
  pub fn new(v: f64) -> Option<Self> {
    if v.is_nan() {
      None
    } else {
      Some(Self(v.to_bits()))
    }
  }

  /// # Safety
  /// `v` must not be nan
  pub unsafe fn new_unchecked(v: f64) -> Self {
    Self(v.to_bits())
  }

  pub fn value(self) -> f64 {
    f64::from_bits(self.0)
  }
}

impl From<NFloat> for f64 {
  fn from(v: NFloat) -> Self {
    v.value()
  }
}

impl TryFrom<f64> for NFloat {
  type Error = ();

  fn try_from(value: f64) -> Result<Self, Self::Error> {
    NFloat::new(value).ok_or(())
  }
}
