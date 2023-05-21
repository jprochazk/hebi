#[macro_use]
mod macros;

use std::collections::HashMap;

use super::*;
use crate as hebi;

check! {
  example,
  r#"
    v := 0
    v
  "#
}

check! {
  make_fn,
  r#"
    fn test():
      return "yo"

    test
  "#
}

check! {
  basic_fn_call,
  r#"
    fn test():
      return "yo"

    test()
  "#
}

check! {
  closure_call,
  r#"
    fn outer():
      v := "yo"
      fn inner():
        return v
      return inner
    
    outer()()
  "#
}

check! {
  make_fn_with_args,
  r#"
    fn test(v):
      return v
    
    test
  "#
}

check! {
  call_fn_with_args,
  r#"
    fn test(v):
      return v
    
    test(100)
  "#
}

check! {
  call_fn_with_args__error_not_enough_args,
  r#"
    fn test(v):
      return v
    
    test()
  "#
}

check! {
  call_fn_with_args__error_too_many_args,
  r#"
    fn test(v):
      return v
    
    test(100, 100)
  "#
}

check! {
  call_fn_recursive,
  r#"
    fn test(v):
      if v: return "yo"
      else: return test(true)
    
    test(true)
    test(false)
  "#
}

struct TestModuleLoader {
  modules: HashMap<&'static str, &'static str>,
}

impl TestModuleLoader {
  pub fn new(modules: &[(&'static str, &'static str)]) -> Self {
    Self {
      modules: HashMap::from_iter(modules.iter().cloned()),
    }
  }
}

impl module::Loader for TestModuleLoader {
  fn load(&self, path: &str) -> hebi::Result<&str> {
    match self.modules.get(path).copied() {
      Some(module) => Ok(module),
      None => Err(Error::Vm(SpannedError::new(
        format!("module `{path}` not found"),
        None,
      ))),
    }
  }
}

check! {
  module
  import_value,
  {
    test: "value := 100"
  },
  r#"
    import test
    test.value
  "#
}

check! {
  module
  import_value_named,
  {
    test: "value := 100"
  },
  r#"
    from test import value
    value
  "#
}

check! {
  module
  import_fn,
  {
    test: r#"
      fn test(value):
        return value
    "#
  },
  r#"
    import test
    test.test("yo")
  "#
}

check! {
  module
  import_fn_named,
  {
    test: r#"
      fn test(value):
        return value
    "#
  },
  r#"
    from test import test
    test("yo")
  "#
}

check! {
  module
  module_vars,
  {
    test: r#"
      value := 100
      fn set(v):
        value = v
      fn get():
        return value
    "#
  },
  r#"
    import test
    
    test.set(50)
    test.get()
  "#
}

check! {
  module
  module_vars_per_module,
  {
    test: r#"
      value := 100
      fn set(v):
        value = v
      fn get():
        return value
    "#
  },
  r#"
    import test

    value := 200
    test.set(50)
    value = 0
    test.get()
  "#
}

check! {
  module
  module_fail_to_parse,
  {
    test: r#"
      fn invalid:
        pass
    "#
  },
  r#"
    import test
  "#
}

check! {
  module
  module_not_found,
  {},
  r#"
    import test
  "#
}

check! {
  simple_class,
  r#"
    class T: pass
    T
  "#
}

check! {
  simple_class_derived,
  r#"
    class T: pass
    class U(T): pass
    U
  "#
}

check! {
  class_with_method,
  r#"
    class T:
      fn test(self): pass
    T
  "#
}

check! {
  class_with_multiple_methods,
  r#"
    class T:
      fn test_0(self): pass
      fn test_1(self): pass
    T
  "#
}

check! {
  class_derived_with_method,
  r#"
    class T:
      pass
    class U(T):
      fn test(self): pass
    U
  "#
}

check! {
  class_derived_with_multiple_methods,
  r#"
    class T: pass
    class U(T):
      fn test_0(self): pass
      fn test_1(self): pass
    U
  "#
}

check! {
  class_derived_with_parent_method,
  r#"
    class T:
      fn test(self): pass
    class U(T): pass
    U
  "#
}

check! {
  class_derived_with_parent_multiple_methods,
  r#"
    class T:
      fn test_0(self): pass
      fn test_1(self): pass
    class U(T): pass
    U
  "#
}

check! {
  simple_data_class,
  r#"
    class T:
      v = 0
    T
  "#
}

check! {
  simple_data_class_derived,
  r#"
    class T: pass
    class U(T):
      v = 0
    U
  "#
}

check! {
  simple_data_class_derived_with_parent_field,
  r#"
    class T:
      v = 0
    class U(T): pass
    U
  "#
}

check! {
  data_class_with_method,
  r#"
    class T:
      v = 0
      fn test(self): pass
    T
  "#
}

check! {
  data_class_with_multiple_methods,
  r#"
    class T:
      v = 0
      fn test_0(self): pass
      fn test_1(self): pass
    T
  "#
}

check! {
  data_class_derived_with_method,
  r#"
    class T:
      pass
    class U(T):
      v = 0
      fn test(self): pass
    U
  "#
}

check! {
  data_class_derived_with_multiple_methods,
  r#"
    class T: pass
    class U(T):
      v = 0
      fn test_0(self): pass
      fn test_1(self): pass
    U
  "#
}

check! {
  data_class_derived_with_parent_method,
  r#"
    class T:
      v = 0
      fn test(self): pass
    class U(T): pass
    U
  "#
}

check! {
  data_class_derived_with_parent_multiple_methods,
  r#"
    class T:
      v = 0
      fn test_0(self): pass
      fn test_1(self): pass
    class U(T): pass
    U
  "#
}

check! {
  init_simple_class,
  r#"
    class T: pass
    T()
  "#
}

check! {
  init_simple_class_derived,
  r#"
    class T: pass
    class U(T): pass
    U()
  "#
}

check! {
  init_class_with_method,
  r#"
    class T:
      fn test(self): pass
    T()
  "#
}

check! {
  init_class_with_multiple_methods,
  r#"
    class T:
      fn test_0(self): pass
      fn test_1(self): pass
    T()
  "#
}

check! {
  init_class_derived_with_method,
  r#"
    class T:
      pass
    class U(T):
      fn test(self): pass
    U()
  "#
}

check! {
  init_class_derived_with_multiple_methods,
  r#"
    class T: pass
    class U(T):
      fn test_0(self): pass
      fn test_1(self): pass
    U()
  "#
}

check! {
  init_class_derived_with_parent_method,
  r#"
    class T:
      fn test(self): pass
    class U(T): pass
    U()
  "#
}

check! {
  init_class_derived_with_parent_multiple_methods,
  r#"
    class T:
      fn test_0(self): pass
      fn test_1(self): pass
    class U(T): pass
    U()
  "#
}

check! {
  init_simple_data_class,
  r#"
    class T:
      v = 0
    T()
  "#
}

check! {
  init_simple_data_class_derived,
  r#"
    class T: pass
    class U(T):
      v = 0
    U()
  "#
}

check! {
  init_simple_data_class_derived_with_parent_field,
  r#"
    class T:
      v = 0
    class U(T): pass
    U()
  "#
}

check! {
  init_data_class_with_method,
  r#"
    class T:
      v = 0
      fn test(self): pass
    T()
  "#
}

check! {
  init_data_class_with_multiple_methods,
  r#"
    class T:
      v = 0
      fn test_0(self): pass
      fn test_1(self): pass
    T()
  "#
}

check! {
  init_data_class_derived_with_method,
  r#"
    class T:
      pass
    class U(T):
      v = 0
      fn test(self): pass
    U()
  "#
}

check! {
  init_data_class_derived_with_multiple_methods,
  r#"
    class T: pass
    class U(T):
      v = 0
      fn test_0(self): pass
      fn test_1(self): pass
    U()
  "#
}

check! {
  init_data_class_derived_with_parent_method,
  r#"
    class T:
      v = 0
      fn test(self): pass
    class U(T): pass
    U()
  "#
}

check! {
  init_data_class_derived_with_parent_multiple_methods,
  r#"
    class T:
      v = 0
      fn test_0(self): pass
      fn test_1(self): pass
    class U(T): pass
    U()
  "#
}

check! {
  class_with_init,
  r#"
    class T:
      fn init(self):
        self.v = 10
    T().v
  "#
}

check! {
  class_with_init_conditional_false,
  r#"
    class T:
      fn init(self, v, set):
        if set:
          self.v = v
    ?T(10, false).v
  "#
}

check! {
  class_with_init_conditional_true,
  r#"
    class T:
      fn init(self, v, set):
        if set:
          self.v = v
    ?T(10, true).v
  "#
}

check! {
  get_class_method,
  r#"
    class T:
      fn test(self):
        pass
    T().test
  "#
}

check! {
  get_class_field,
  r#"
    class T:
      v = 10
    T().v
  "#
}

check! {
  call_class_method,
  r#"
    class T:
      v = 10
      fn test(self):
        return self.v
    T().test()
  "#
}

check! {
  call_class_method2,
  r#"
    class T:
      v = 10
      fn set(self, v):
        self.v = v
      fn get(self):
        return self.v

    t := T()
    t.set(20)
    t.get()
  "#
}

check! {
  call_class_method_derived,
  r#"
    class T:
      v = 0
      fn test(self):
        return self.v
    class U(T):
      fn test(self):
        return super.test()
    U().test()
  "#
}

check! {
  call_class_method_derived2,
  r#"
    class T:
      fn test(self, v):
        return v
    class U(T):
      fn test(self, v):
        return super.test(v)
    U().test(10)
  "#
}

check! {
  call_class_method_derived3,
  r#"
    class T:
      fn test(self, v=5):
        return v
    class U(T):
      fn test(self, v):
        return super.test(v)
    U().test(none)
  "#
}

check! {
  call_class_method_derived4,
  r#"
    class T:
      fn test(self, v):
        return v
    class U(T):
      fn test(self, v=5):
        return super.test(v)
    U().test()
  "#
}

check! {
  call_class_method_derived_static,
  r#"
    class T:
      fn test(self, v):
        return v
    class U(T):
      fn test(self, v=5):
        return super.test(v)
    U.test(U())
  "#
}

check! {
  call_class_method_derived_static2,
  r#"
    class T:
      fn test(self, v):
        return v
    class U(T):
      fn test(self, v=5):
        return super.test(v)
    U.test(U(), 10)
  "#
}

check! {
  call_class_nested_inheritance_method,
  r#"
    class A:
      fn test(self, v):
        return v + 1
    class B(A):
      fn test(self, v):
        return super.test(v) + 1
    class C(B):
      fn test(self, v):
        return super.test(v) + 1
    
    C().test(0)
  "#
}

check! {
  call_class_nested_inheritance_method_static_call,
  r#"
    class A:
      fn test(self, v):
        return v + 1
    class B(A):
      fn test(self, v):
        return super.test(v) + 1
    class C(B):
      fn test(self, v):
        return super.test(v) + 1
    
    C.test(C(), 0)
  "#
}

#[tokio::test]
async fn subsequent_eval() {
  let mut hebi = Vm::new();
  hebi.eval("v := 0").await.unwrap();
  let value = hebi.eval("v").await.unwrap().to_int();
  assert_eq!(value, Some(0));
}

check! {
  nested_optional_access,
  r#"
    v := none

    ?v.a["b"].c ?? "test"
  "#
}

check! {
  empty_table,
  r#"
    v := {}
    v
  "#
}

check! {
  nested_table,
  r#"
    v := {a: {b: 10}}
    v
  "#
}

check! {
  table_access_named,
  r#"
    v := {a: 10}
    v.a
  "#
}

check! {
  table_access_keyed,
  r#"
    v := {a: 10}
    v["a"]
  "#
}

check! {
  table_nested_access_named,
  r#"
    v := {a: {b: 10}}
    v.a.b
  "#
}

check! {
  table_nested_access_keyed,
  r#"
    v := {a: {b: 10}}
    v["a"]["b"]
  "#
}

check! {
  arithmetic,
  r#"
    v := 10 # 10
    v += 1  # 11
    v -= 1  # 10
    v **= 2 # 100
    v /= 5  # 20
    v %= 1  # 0
    v
  "#
}

check! {
  unary_invert,
  r#"
    v := 20
    -v
  "#
}

check! {
  unary_not,
  r#"
    v := false
    !v
  "#
}

check! {
  unary_not_int,
  r#"
    v := 0
    !v
  "#
}

check! {
  unary_not_float,
  r#"
    v := 0.0
    !v
  "#
}

check! {
  unary_not_none,
  r#"
    v := none
    !v
  "#
}

check! {
  unary_not_str,
  r#"
    v := "test"
    !v
  "#
}

check! {
  if_stmt,
  r#"
    v := true
    result := none
    if v:
      result = "true"
    else:
      result = "false"
    result
  "#
}

check! {
  if_stmt_false,
  r#"
    v := false
    result := none
    if v:
      result = "true"
    else:
      result = "false"
    result
  "#
}

check! {
  more_optional_access,
  r#"
    "a" ?? "b"
  "#
}

check! {
  logical_or_expr_return_lhs,
  r#"
    "a" || "b"
  "#
}

check! {
  logical_or_expr_return_rhs,
  r#"
    false || "b"
  "#
}

check! {
  logical_and_expr_return_rhs,
  r#"
    "a" && "b"
  "#
}

check! {
  logical_and_expr_return_lhs,
  r#"
    false && "b"
  "#
}

/*
TODO: more tests

- loops (for, while, loop)
  - continue, break
- native modules
  - functions
  - async functions
  - classes
  - class methods
  - async class methods
  - class methods (static call)
  - async class methods (static call)
  - class static methods
  - class static async methods
*/
