check! {
  empty_class,
  r#"
    class T: pass
    print T
  "#
}

check! {
  empty_class_with_super,
  r#"
    class U: pass
    class T(U): pass
    print T
  "#
}

check! {
  class_with_field,
  r#"
    class T:
      v = 10

    t0 = T()
    t1 = T(v=20)

    print t0.v, t1.v
  "#
}

check! {
  class_with_init,
  r#"
    class T:
      fn init(self, *, v=10):
        self.v = v
        if v > 10:
          print "large"
        else:
          print "small"

    t0 := T()
    t1 := T(v=20)

    print t0.v, t1.v
  "#
}
