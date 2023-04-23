pub struct RegAlloc {
  preserve: Vec<Option<Register>>,
}

pub struct Register(usize);

impl RegAlloc {
  pub fn new() -> Self {
    Self {
      preserve: Vec::new(),
    }
  }
}
