use derive::{class, methods};

#[class]
struct Test0 {}

check! {
  empty_class,
  classes: [Test0],
  r#"
    print Test0
  "#
}

check_error! {
  cannot_inherit_native,
  classes: [Test0],
  r#"
    class Bad(Test0):
      pass
  "#
}

#[class]
struct Test1 {
  #[getset]
  v: i32,
}

#[methods]
impl Test1 {
  #[init]
  pub fn new(
    #[default(10)]
    #[kw]
    v: i32,
  ) -> Self {
    Self { v }
  }
}

check! {
  class_with_init_and_field,
  classes: [Test1],
  r#"
    t0 = Test1()
    t1 = Test1(v=20)

    print t0.v, t1.v

    t0.v = t1.v

    print t0.v, t1.v
  "#
}

#[class]
struct WithMethods {}

#[methods]
impl WithMethods {
  #[init]
  pub fn new() -> Self {
    Self {}
  }

  #[allow(dead_code)]
  fn not_exposed(&self) -> &'static str {
    "not exposed"
  }

  pub fn exposed(&self) -> &'static str {
    "exposed"
  }
}

check! {
  class_method_call,
  classes: [WithMethods],
  r#"
    v := WithMethods()
    print v.exposed()
  "#
}

check_error! {
  class_method_call_not_exposed,
  classes: [WithMethods],
  r#"
    v := WithMethods()
    print v.not_exposed()
  "#
}

#[class]
struct Calls {}

#[methods]
impl Calls {
  #[init]
  pub fn new() -> Self {
    Self {}
  }

  pub fn simple(&self) -> i32 {
    0
  }

  pub fn with_pos(&self, v: i32) -> i32 {
    v
  }

  pub fn with_kw(&self, #[kw] v: i32) -> i32 {
    v
  }

  pub fn with_pos_default(&self, a: i32, #[default(100)] b: i32) -> i32 {
    a + b
  }

  pub fn with_kw_default(
    &self,
    #[kw] a: i32,
    #[default(100)]
    #[kw]
    b: i32,
  ) -> i32 {
    a + b
  }

  pub fn static_simple() -> i32 {
    0
  }

  pub fn static_with_pos(v: i32) -> i32 {
    v
  }

  pub fn static_with_kw(#[kw] v: i32) -> i32 {
    v
  }

  pub fn static_with_pos_default(a: i32, #[default(100)] b: i32) -> i32 {
    a + b
  }

  pub fn static_with_kw_default(
    #[kw] a: i32,
    #[default(100)]
    #[kw]
    b: i32,
  ) -> i32 {
    a + b
  }
}

check! {
  method_calls,
  classes: [Calls],
  r#"
    v := Calls()

    print "simple:", v.simple()
    print "pos:", v.with_pos(10)
    print "kw:", v.with_kw(v=10)
    print "pos default:", v.with_pos_default(10)
    print "pos default with 2nd arg:", v.with_pos_default(10, -10)
    print "kw default:", v.with_kw_default(a=10)
    print "kw default with 2nd arg:", v.with_kw_default(a=10, b=-10)
  "#
}

check! {
  static_method_calls,
  classes: [Calls],
  r#"
    print "static + simple:", Calls.static_simple()
    print "static + pos:", Calls.static_with_pos(10)
    print "static + kw:", Calls.static_with_kw(v=10)
    print "static + pos default:", Calls.static_with_pos_default(10)
    print "static + pos default with 2nd arg:", Calls.static_with_pos_default(10, -10)
    print "static + kw default:", Calls.static_with_kw_default(a=10)
    print "static + kw default with 2nd arg:", Calls.static_with_kw_default(a=10, b=-10)
  "#
}

#[class]
struct Mutable {
  #[getset]
  value: i32,
}

#[methods]
impl Mutable {
  #[init]
  pub fn new() -> Self {
    Self { value: 0 }
  }

  pub fn add(&mut self, v: i32) {
    self.value += v;
  }
}

check! {
  mutate_through_method,
  classes: [Mutable],
  r#"
    m := Mutable()
    print m.value
    m.add(10)
    print m.value
  "#
}
