use std::rc::Rc;

pub struct Ptr(pub(crate) Rc<Temp>);

impl Ptr {
  pub(crate) fn new() -> Self {
    Ptr(Rc::new(Temp(0u64)))
  }
}

// Temporary object representation. This should probably be a trait.
pub(crate) struct Temp(pub u64);

impl std::fmt::Debug for Ptr {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self.0.as_ref())
  }
}

impl std::fmt::Display for Ptr {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    std::fmt::Display::fmt(&self.0, f)
  }
}

impl std::fmt::Debug for Temp {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<object>")
  }
}

impl std::fmt::Display for Temp {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<object>")
  }
}

impl Drop for Temp {
  fn drop(&mut self) {
    println!("temp dropped");
  }
}
