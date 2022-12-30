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

  pub fn is_indent_eq(&self, n: u64) -> bool {
    println!("{} == {}", self.level, n);
    self.level == n
  }

  pub fn is_indent_gt(&self, n: u64) -> bool {
    println!("{} < {}", self.level, n);
    self.level < n
  }

  pub fn is_indent_lt(&self, n: u64) -> bool {
    println!("{} > {}", self.level, n);
    self.level > n
  }

  pub fn push_indent(&mut self, n: u64) {
    self.stack.push(n);
    self.level = n;
  }

  pub fn pop_indent(&mut self) {
    self.stack.pop().unwrap();
    self.level = self
      .stack
      .last()
      .cloned()
      .expect("pop_indent should not empty the indent stack");
  }
}
