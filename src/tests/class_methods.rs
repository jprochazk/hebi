check! {
  class_bound_method,
  r#"
    class T:
      v = 10
      fn test(self):
        print self.v
    
    t0 := T()
    t1 := T(v=20)
    t0.test()
    t1.test()
    t0f := t0.test
    t1f := t1.test
    t0f()
    t1f()
  "#
}

check! {
  method_call,
  r#"
    class T:
      fn test():
        print "test"

    T.test()
  "#
}

check! {
  class_call_init,
  r#"
    class A:
      v = 10
    class B:
      fn init(self, *items):
        self.items = items
    class C:
      fn init(self, **entries):
        self.entries = entries

    print A().v
    print B(0, 1, 2).items
    print C(a=0, b=1, c=2).entries
  "#
}

check! {
  method_receiver_resolution,
  r#"
    class A:
      v = 10
      fn test(self):
        print self.v
    

    fn test():
      print "test"

    class B:
      test = test

    A().test() # params_base = 4 + implicit receiver
    A.test(A()) # params_base = 3
    test() # params_base = 4
    B().test() # params_base = 4 + implicit receiver
  "#
}
