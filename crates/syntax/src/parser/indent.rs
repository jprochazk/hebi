pub struct IndentStack {
  stack: Vec<u64>,
  level: u64,
}

impl IndentStack {
  pub fn new() -> Self {
    Self {
      stack: vec![0],
      level: 0,
    }
  }

  pub fn is_eq(&self, n: u64) -> bool {
    self.level == n
  }

  pub fn is_gt(&self, n: u64) -> bool {
    self.level < n
  }

  pub fn is_lt(&self, n: u64) -> bool {
    self.level > n
  }

  pub fn push(&mut self, n: u64) {
    self.stack.push(n);
    self.level = n;
  }

  pub fn pop(&mut self) {
    self.stack.pop().unwrap();
    self.level = self
      .stack
      .last()
      .cloned()
      .expect("pop_indent should not empty the indent stack");
  }

  pub fn reset(&mut self) {
    self.stack.clear();
    self.stack.push(0);
    self.level = 0;
  }
}
