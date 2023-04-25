#[derive(Default)]
pub struct RegAlloc {
  preserve: Vec<Option<Register>>,
  intervals: Vec<Interval>,
  event: usize,
}

struct Interval {
  register: Register,
  start: usize,
  end: usize,
}

#[derive(Clone, Copy)]
pub struct Register(usize);

impl RegAlloc {
  pub fn new() -> Self {
    Self {
      preserve: Vec::new(),
      intervals: Vec::new(),
      event: 0,
    }
  }

  fn event(&mut self) -> usize {
    let event = self.event;
    self.event += 1;
    event
  }

  pub fn alloc(&mut self) -> Register {
    let register = Register(self.intervals.len());
    let event = self.event();
    self.intervals.push(Interval {
      register,
      start: event,
      end: event,
    });
    register
  }
}
