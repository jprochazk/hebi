#[path = "common/common.rs"]
#[macro_use]
mod common;
check! {
  class_inheritance_nested,
  :print_func
  r#"
    class A:
      v = 0
      fn test(self):
        print "A", self.v
    class B(A):
      v = 1
      fn test(self):
        super.test()
        print "B", self.v
    class C(B):
      v = 2
      fn test(self):
        super.test()
        print "C", self.v
    
    A().test()
    B().test()
    C().test()
  "#
}
