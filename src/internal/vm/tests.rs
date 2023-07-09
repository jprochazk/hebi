#[macro_use]
mod macros;

use std::collections::HashMap;

use super::*;
use crate::public::Scope;

check! {
  example,
  r#"#!hebi
    v := 0
    v
  "#
}

check! {
  make_fn,
  r#"#!hebi
    fn test():
      return "yo"

    test
  "#
}

check! {
  basic_fn_call,
  r#"#!hebi
    fn test():
      return "yo"

    test()
  "#
}

check! {
  closure_call,
  r#"#!hebi
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
  r#"#!hebi
    fn test(v):
      return v
    
    test
  "#
}

check! {
  call_fn_with_args,
  r#"#!hebi
    fn test(v):
      return v
    
    test(100)
  "#
}

check! {
  call_fn_with_args__error_not_enough_args,
  r#"#!hebi
    fn test(v):
      return v
    
    test()
  "#
}

check! {
  call_fn_with_args__error_too_many_args,
  r#"#!hebi
    fn test(v):
      return v
    
    test(100, 100)
  "#
}

check! {
  call_fn_recursive,
  r#"#!hebi
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

impl module::ModuleLoader for TestModuleLoader {
  fn load(&self, path: &str) -> Result<Cow<'static, str>> {
    match self.modules.get(path).copied() {
      Some(module) => Ok(Cow::borrowed(module)),
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
  r#"#!hebi
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
  r#"#!hebi
    from test import value
    value
  "#
}

check! {
  module
  use_import_in_nested_scope,
  {
    test: "value := 100"
  },
  r#"#!hebi
    import test
    fn foo():
      print test
    foo()
  "#
}

check! {
  module
  use_named_import_in_nested_scope,
  {
    test: "value := 100"
  },
  r#"#!hebi
    from test import value
    fn foo():
      print value
    foo()
  "#
}

check! {
  module
  import_fn,
  {
    test: r#"#!hebi
      fn test(value):
        return value
    "#
  },
  r#"#!hebi
    import test
    test.test("yo")
  "#
}

check! {
  module
  import_fn_named,
  {
    test: r#"#!hebi
      fn test(value):
        return value
    "#
  },
  r#"#!hebi
    from test import test
    test("yo")
  "#
}

check! {
  module
  module_vars,
  {
    test: r#"#!hebi
      value := 100
      fn set(v):
        value = v
      fn get():
        return value
    "#
  },
  r#"#!hebi
    import test
    
    test.set(50)
    test.get()
  "#
}

check! {
  module
  module_vars_per_module,
  {
    test: r#"#!hebi
      value := 100
      fn set(v):
        value = v
      fn get():
        return value
    "#
  },
  r#"#!hebi
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
    test: r#"#!hebi
      fn invalid:
        pass
    "#
  },
  r#"#!hebi
    import test
  "#
}

check! {
  module
  module_not_found,
  {},
  r#"#!hebi
    import test
  "#
}

check! {
  simple_class,
  r#"#!hebi
    class T: pass
    T
  "#
}

check! {
  simple_class_derived,
  r#"#!hebi
    class T: pass
    class U(T): pass
    U
  "#
}

check! {
  class_with_method,
  r#"#!hebi
    class T:
      fn test(self): pass
    T
  "#
}

check! {
  class_with_multiple_methods,
  r#"#!hebi
    class T:
      fn test_0(self): pass
      fn test_1(self): pass
    T
  "#
}

check! {
  class_derived_with_method,
  r#"#!hebi
    class T:
      pass
    class U(T):
      fn test(self): pass
    U
  "#
}

check! {
  class_derived_with_multiple_methods,
  r#"#!hebi
    class T: pass
    class U(T):
      fn test_0(self): pass
      fn test_1(self): pass
    U
  "#
}

check! {
  class_derived_with_parent_method,
  r#"#!hebi
    class T:
      fn test(self): pass
    class U(T): pass
    U
  "#
}

check! {
  class_derived_with_parent_multiple_methods,
  r#"#!hebi
    class T:
      fn test_0(self): pass
      fn test_1(self): pass
    class U(T): pass
    U
  "#
}

check! {
  simple_data_class,
  r#"#!hebi
    class T:
      v = 0
    T
  "#
}

check! {
  simple_data_class_derived,
  r#"#!hebi
    class T: pass
    class U(T):
      v = 0
    U
  "#
}

check! {
  simple_data_class_derived_with_parent_field,
  r#"#!hebi
    class T:
      v = 0
    class U(T): pass
    U
  "#
}

check! {
  data_class_with_method,
  r#"#!hebi
    class T:
      v = 0
      fn test(self): pass
    T
  "#
}

check! {
  data_class_with_multiple_methods,
  r#"#!hebi
    class T:
      v = 0
      fn test_0(self): pass
      fn test_1(self): pass
    T
  "#
}

check! {
  data_class_derived_with_method,
  r#"#!hebi
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
  r#"#!hebi
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
  r#"#!hebi
    class T:
      v = 0
      fn test(self): pass
    class U(T): pass
    U
  "#
}

check! {
  data_class_derived_with_parent_multiple_methods,
  r#"#!hebi
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
  r#"#!hebi
    class T: pass
    T()
  "#
}

check! {
  init_simple_class_derived,
  r#"#!hebi
    class T: pass
    class U(T): pass
    U()
  "#
}

check! {
  init_class_with_method,
  r#"#!hebi
    class T:
      fn test(self): pass
    T()
  "#
}

check! {
  init_class_with_multiple_methods,
  r#"#!hebi
    class T:
      fn test_0(self): pass
      fn test_1(self): pass
    T()
  "#
}

check! {
  init_class_derived_with_method,
  r#"#!hebi
    class T:
      pass
    class U(T):
      fn test(self): pass
    U()
  "#
}

check! {
  init_class_derived_with_multiple_methods,
  r#"#!hebi
    class T: pass
    class U(T):
      fn test_0(self): pass
      fn test_1(self): pass
    U()
  "#
}

check! {
  init_class_derived_with_parent_method,
  r#"#!hebi
    class T:
      fn test(self): pass
    class U(T): pass
    U()
  "#
}

check! {
  init_class_derived_with_parent_multiple_methods,
  r#"#!hebi
    class T:
      fn test_0(self): pass
      fn test_1(self): pass
    class U(T): pass
    U()
  "#
}

check! {
  init_simple_data_class,
  r#"#!hebi
    class T:
      v = 0
    T()
  "#
}

check! {
  init_simple_data_class_derived,
  r#"#!hebi
    class T: pass
    class U(T):
      v = 0
    U()
  "#
}

check! {
  init_simple_data_class_derived_with_parent_field,
  r#"#!hebi
    class T:
      v = 0
    class U(T): pass
    U()
  "#
}

check! {
  init_data_class_with_method,
  r#"#!hebi
    class T:
      v = 0
      fn test(self): pass
    T()
  "#
}

check! {
  init_data_class_with_multiple_methods,
  r#"#!hebi
    class T:
      v = 0
      fn test_0(self): pass
      fn test_1(self): pass
    T()
  "#
}

check! {
  init_data_class_derived_with_method,
  r#"#!hebi
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
  r#"#!hebi
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
  r#"#!hebi
    class T:
      v = 0
      fn test(self): pass
    class U(T): pass
    U()
  "#
}

check! {
  init_data_class_derived_with_parent_multiple_methods,
  r#"#!hebi
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
  r#"#!hebi
    class T:
      v = 0
      init(self):
        self.v = 10
    T().v
  "#
}

check! {
  class_with_init_conditional_false,
  r#"#!hebi
    class T:
      v = none
      init(self, v, set):
        if set:
          self.v = v
    ?T(10, false).v
  "#
}

check! {
  class_with_init_conditional_true,
  r#"#!hebi
    class T:
      v = none
      init(self, v, set):
        if set:
          self.v = v
    ?T(10, true).v
  "#
}

check! {
  class_derived_with_init,
  r#"#!hebi
    class T:
      pass
    class U(T):
      init(self):
        print("U.init")
    _ := U()
  "#
}

check! {
  class_derived_with_parent_init,
  r#"#!hebi
    class T:
      init(self):
        print("T.init")
    class U(T):
      pass
    _ := U()
  "#
}

check! {
  class_derived_with_init_and_no_call_parent_init,
  r#"#!hebi
    class T:
      init(self):
        print("T.init")
    class U(T):
      init(self):
        print("U.init")
    _ := U()
  "#
}

check! {
  class_derived_with_init_and_call_parent_init,
  r#"#!hebi
    class T:
      init(self):
        print("T.init")
    class U(T):
      init(self):
        super()
        print("U.init")
    _ := U()
  "#
}

check! {
  class_derived_nested_init_call_chain,
  r#"#!hebi
    class T:
      init(self):
        print("T.init")
    class U(T):
      init(self):
        super()
        print("U.init")
    class V(U):
      init(self):
        super()
        print("V.init")
    _ := V()
  "#
}

check! {
  get_class_method,
  r#"#!hebi
    class T:
      fn test(self):
        pass
    T().test
  "#
}

check! {
  get_class_parent_method,
  r#"#!hebi
    class T:
      fn test(self):
        pass
    class U(T):
      pass
    U().test
  "#
}

check! {
  get_class_field,
  r#"#!hebi
    class T:
      v = 10
    T().v
  "#
}

check! {
  get_class_parent_field,
  r#"#!hebi
    class T:
      v = 10
    class U(T):
      pass
    U().v
  "#
}

check! {
  call_class_method,
  r#"#!hebi
    class T:
      v = 10
      fn test(self):
        return self.v
    T().test()
  "#
}

check! {
  call_class_method2,
  r#"#!hebi
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
  r#"#!hebi
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
  r#"#!hebi
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
  r#"#!hebi
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
  r#"#!hebi
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
  r#"#!hebi
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
  r#"#!hebi
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
  r#"#!hebi
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
  r#"#!hebi
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
  let mut hebi = Vm::default();
  hebi.eval("v := 0").await.unwrap();
  let value = hebi.eval("v").await.unwrap().to_int();
  assert_eq!(value, Some(0));
}

#[tokio::test]
async fn subsequent_eval_with_error() {
  fn error(_: Scope<'_>) -> Result<()> {
    fail!("explicit failure")
  }

  let mut hebi = Vm::default();
  hebi.register(
    &NativeModule::builder("test")
      .function("error", error)
      .finish(),
  );

  let source = indoc::indoc!(
    r#"#!hebi
      from test import error

      fn inner():
        error()
      
      inner()
    "#
  );

  eprintln!("{}", hebi.compile(source).unwrap().disassemble());

  for _ in 0..10 {
    hebi.eval(source).await.unwrap_err();
  }

  {
    let stack = unsafe { hebi.root.stack.as_ref() };
    assert!(stack.frames.is_empty());
    assert!(stack.regs.is_empty());
  }
}

check! {
  nested_optional_access,
  r#"#!hebi
    v := none

    ?v.a["b"].c ?? "test"
  "#
}

check! {
  empty_table,
  r#"#!hebi
    v := {}
    v
  "#
}

check! {
  nested_table,
  r#"#!hebi
    v := {a: {b: 10}}
    v
  "#
}

check! {
  table_access_named,
  r#"#!hebi
    v := {a: 10}
    v.a
  "#
}

check! {
  table_access_keyed,
  r#"#!hebi
    v := {a: 10}
    v["a"]
  "#
}

check! {
  table_nested_access_keyed,
  r#"#!hebi
    v := {a: {b: 10}}
    v["a"]["b"]
  "#
}

check! {
  table_access_unknown,
  r#"#!hebi
    v := {}
    v["a"]
  "#
}

check! {
  arithmetic,
  r#"#!hebi
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
  r#"#!hebi
    v := 20
    -v
  "#
}

check! {
  unary_not,
  r#"#!hebi
    v := false
    !v
  "#
}

check! {
  unary_not_int,
  r#"#!hebi
    v := 0
    !v
  "#
}

check! {
  unary_not_float,
  r#"#!hebi
    v := 0.0
    !v
  "#
}

check! {
  unary_not_none,
  r#"#!hebi
    v := none
    !v
  "#
}

check! {
  unary_not_str,
  r#"#!hebi
    v := "test"
    !v
  "#
}

check! {
  if_stmt,
  r#"#!hebi
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
  r#"#!hebi
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
  r#"#!hebi
    "a" ?? "b"
  "#
}

check! {
  logical_or_expr_return_lhs,
  r#"#!hebi
    "a" || "b"
  "#
}

check! {
  logical_or_expr_return_rhs,
  r#"#!hebi
    false || "b"
  "#
}

check! {
  logical_and_expr_return_rhs,
  r#"#!hebi
    "a" && "b"
  "#
}

check! {
  logical_and_expr_return_lhs,
  r#"#!hebi
    false && "b"
  "#
}

check! {
  list_indexing_zero,
  r#"#!hebi
    [0, 1, 2][0]
  "#
}

check! {
  list_indexing_positive,
  r#"#!hebi
    [0, 1, 2][1]
  "#
}

check! {
  list_indexing_negative,
  r#"#!hebi
    [0, 1, 2][-1]
  "#
}

check! {
  list_indexing_invalid,
  r#"#!hebi
    [0, 1, 2]["yo"]
  "#
}

check! {
  list_indexing_oob,
  r#"#!hebi
    [0, 1, 2][100]
  "#
}

check! {
  list_indexing_zero_opt,
  r#"#!hebi
    ?[0, 1, 2][0]
  "#
}

check! {
  list_indexing_positive_opt,
  r#"#!hebi
    ?[0, 1, 2][1]
  "#
}

check! {
  list_indexing_negative_opt,
  r#"#!hebi
    ?[0, 1, 2][-1]
  "#
}

check! {
  list_indexing_invalid_opt,
  r#"#!hebi
    ?[0, 1, 2]["yo"]
  "#
}

check! {
  list_indexing_oob_opt,
  r#"#!hebi
    ?[0, 1, 2][100]
  "#
}

check! {
  for_iter_list,
  r#"#!hebi
    for item in ["a", "b", "c"]:
      print item
  "#
}

check! {
  for_iter_iterable_class,
  r#"#!hebi
    class Counter:
      n = 0
      max = 0

      init(self, max):
        self.max = max

      fn iter(self):
        return self

      fn next(self):
        if self.n < self.max:
          n := self.n
          self.n += 1
          return n

      fn done(self):
        return self.n >= self.max

    for v in Counter(10):
      print v
  "#
}

check! {
  builtin_list_methods,
  r#"#!hebi
    v := [0, 1, 2]

    print "len", v.len()
    print "is_empty", v.is_empty()
    print "get(1)", v.get(1)
    print "pop()", v.pop()
    print "set", v.set(0, v.get(1))
    print "join", v.join(", ")
    print "push(2)", v.push(2)
    print "extend(3, 0)", v.extend(3, 0)
    print "join", v.join(", ")
  "#
}

check! {
  builtin_list_methods_bound,
  r#"#!hebi
    v := [0, 1, 2]

    v_len := v.len
    v_is_empty := v.is_empty
    v_get := v.get
    v_pop := v.pop
    v_set := v.set
    v_join := v.join
    v_push := v.push
    v_extend := v.extend
    
    print "len", v_len()
    print "is_empty", v_is_empty()
    print "get(1)", v_get(1)
    print "pop()", v_pop()
    print "set", v_set(0, v_get(1))
    print "join", v_join(", ")
    print "push(2)", v_push(2)
    print "extend(3, 0)", v_extend(3, 0)
    print "join", v_join(", ")
  "#
}

check! {
  builtin_list_methods_static,
  r#"#!hebi
    v := [0, 1, 2]
    
    print "len", List.len(v)
    print "is_empty", List.is_empty(v)
    print "get(1)", List.get(v, 1)
    print "pop()", List.pop(v)
    print "set", List.set(v, 0, List.get(v, 1))
    print "join", List.join(v, ", ")
    print "push(2)", List.push(v, 2)
    print "extend(3, 0)", List.extend(v, 3, 0)
    print "join", List.join(v, ", ")
  "#
}

check! {
  builtin_str_methods,
  r#"#!hebi
    v := "a\nb\nc"

    print "len", v.len()
    print "is_empty", v.is_empty()
    print "lines", v.lines()
  "#
}

check! {
  builtin_str_methods_bound,
  r#"#!hebi
    v := "a\nb\nc"

    v_len := v.len
    v_is_empty := v.is_empty
    v_lines := v.lines

    print "len", v_len()
    print "is_empty", v_is_empty()
    print "lines", v_lines()
  "#
}

check! {
  builtin_str_methods_static,
  r#"#!hebi
    v := "a\nb\nc"

    print "len", Str.len(v)
    print "is_empty", Str.is_empty(v)
    print "lines", Str.lines(v)
  "#
}

check! {
  builtin_str_lines_iter,
  r#"#!hebi
    strings := [
      "a\n\nb\nc",
      "\na\n\nb\nc",
      "\na\n\nb\nc\n",
    ]

    for string in strings:
      for line in string.lines():
        print "`" + line + "`"
      print ""
  "#
}

check! {
  builtin_collect,
  r#"#!hebi
    class Counter:
      n = 0
      max = 0

      init(self, max):
        self.max = max

      fn iter(self):
        return self

      fn next(self):
        if self.n < self.max:
          n := self.n
          self.n += 1
          return n

      fn done(self):
        return self.n >= self.max

    collect(Counter(10))
  "#
}

check! {
  builtin_collect_native,
  r#"#!hebi
    collect("a\nb\nc".lines())
  "#
}

check! {
  builtin_parse_int,
  r#"#!hebi
    print parse_int(10)
    print parse_int(10.0)
    print parse_int("10")
  "#
}

check! {
  add_objects,
  r#"#!hebi
    "a" + "b"
  "#
}

check! {
  string_comparison,
  r#"#!hebi
    print "'a' <  'b'", "a" <  "b"
    print "'b' >= 'a'", "b" >= "a"

    print "'b' <  'a'", "b" <  "a"
    print "'a' >= 'b'", "a" >= "b"

    print "'a' == 'b'", "a" == "b"
    print "'b' == 'a'", "a" == "b"
    print "'a' == 'a'", "a" == "a"
  "#
}

// check! {
//   type_comparison,
//   r#"#!hebi
//     fn interesting_thing(value):
//       if value is Str:
//         print "str: ", value
//       else:
//         print "I hardly know 'er"
//
//     interesting_thing("ppL")
//     interesting_thing(100)
//   "#
// }

check! {
  global_builtin_functions__to_int__float,
  r#"#!hebi
    to_int(10.5)
  "#
}

check! {
  global_builtin_functions__to_int__int,
  r#"#!hebi
    to_int(10)
  "#
}

check! {
  global_builtin_functions__to_int__bad_input,
  r#"#!hebi
    to_int({})
  "#
}

check! {
  global_builtin_functions__to_float__float,
  r#"#!hebi
    to_float(10.5)
  "#
}

check! {
  global_builtin_functions__to_float__int,
  r#"#!hebi
    to_float(10)
  "#
}

check! {
  global_builtin_functions__to_float__bad_input,
  r#"#!hebi
    to_float({})
  "#
}

check! {
  global_builtin_functions__to_bool,
  r#"#!hebi
    print "true", to_bool(true)
    print "false", to_bool(false)
    print "10.0", to_bool(10.0)
    print "0.0", to_bool(0.0)
    print "100", to_bool(100)
    print "0", to_bool(0)
    print "none", to_bool(none)
    print "{}", to_bool({})
    print "[]", to_bool([])
    print "\"test\"", to_bool("test")
  "#
}

check! {
  global_builtin_functions__to_str,
  r#"#!hebi
    print to_str(true)
    print to_str(false)
    print to_str(100)
    print to_str(3.14)
    print to_str(none)
    print to_str({})
    print to_str([])
    print to_str("test")
  "#
}

check! {
  global_builtin_functions__type_of,
  r#"#!hebi
    print type_of(true)
    print type_of(false)
    print type_of(100)
    print type_of(3.14)
    print type_of(none)
    print type_of({})
    print type_of([])
    print type_of("test")
  "#
}

check! {
  make_large_table,
  r#"#!hebi
    {
      version: "0.3.3",
      update_time: 1684753441.253474,
      data: [
        {
          flight_id: "306364ca",
          flight: none,
          callsign: "REDEYE6",
          squawk: none,
          clicks: 1092,
          from_iata: "RMS",
          from_city: "Ramstein",
          to_iata: none,
          to_city: none,
          model: "B703",
          type: "Boeing E-8C"
        }
      ]
    }
  "#
}

check! {
  module
  regression__variable_scope_ends_too_early,
  {
    http: r#"#!hebi
      fn fetch(url, opts):
        return {
          version: "0.3.3",
          update_time: 1684753441.253474,
          data: [
            {
              flight_id: "306364ca",
              flight: none,
              callsign: "REDEYE6",
              squawk: none,
              clicks: 1092,
              from_iata: "RMS",
              from_city: "Ramstein",
              to_iata: none,
              to_city: none,
              model: "B703",
              type: "Boeing E-8C"
            }
          ]
        }
    "#,
    utils: r#"#!hebi
      fn get_element(list, index):
        return list[index]
      fn format(fmt, a, b):
        return fmt
      fn len(list):
        return 1
      fn join(list, sep):
        return "joined"
      fn push(list, item):
        pass
    "#
  },
  r#"#!hebi
    from http import fetch
    from utils import get_element, format, len, join, push

    top_flights := fetch("https://flightradar24.com/flights/most-tracked", {format: "json"})
    data := top_flights["data"]

    i := 0
    flights := []

    while i < len(data):
      flight = get_element(data, i)

      callsign := ?flight["callsign"] ?? "Unknown"
      from_city := ?flight["from_city"] ?? "N/A"
      to_city := ?flight["to_city"] ?? "N/A"

      __format = format
      flight_info := format("{} {}", from_city, to_city)
      print(flight_info)

      i = i + 1

    join(flights, ", ")
    __format # should be the `format` function
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

check! {
  order_of_eval,
  r#"#!hebi
    a := 1
    fn f():
      a += 1
      return a
    print (a + f())
  "#
}

check! {
  bool_equality,
  r#"#!hebi
    a := true
    b := false
    if a == b:
      print "a == b"
    if a != b:
      print "a != b"
  "#
}

check! {
  none_equality,
  r#"#!hebi
    a := none
    if a == none:
      print "a == none"
    if a != none:
      print "a != none"
  "#
}

check! {
  bool_add_error,
  r#"#!hebi
    true + false
  "#
}

check! {
  none_add_error,
  r#"#!hebi
    none + none
  "#
}

check! {
  bool_sub_error,
  r#"#!hebi
    true - false
  "#
}

check! {
  none_sub_error,
  r#"#!hebi
    none - none
  "#
}

check! {
  bool_mul_error,
  r#"#!hebi
    true * false
  "#
}

check! {
  none_mul_error,
  r#"#!hebi
    none * none
  "#
}

check! {
  bool_div_error,
  r#"#!hebi
    true / false
  "#
}

check! {
  none_div_error,
  r#"#!hebi
    none / none
  "#
}

check! {
  bool_rem_error,
  r#"#!hebi
    true % false
  "#
}

check! {
  none_rem_error,
  r#"#!hebi
    none % none
  "#
}

check! {
  bool_pow_error,
  r#"#!hebi
    true ** false
  "#
}

check! {
  none_pow_error,
  r#"#!hebi
    none ** none
  "#
}

check! {
  bool_gt_error,
  r#"#!hebi
    true > false
  "#
}

check! {
  none_gt_error,
  r#"#!hebi
    none > none
  "#
}

check! {
  bool_ge_error,
  r#"#!hebi
    true >= false
  "#
}

check! {
  none_ge_error,
  r#"#!hebi
    none >= none
  "#
}

check! {
  bool_lt_error,
  r#"#!hebi
    true < false
  "#
}

check! {
  none_lt_error,
  r#"#!hebi
    none < none
  "#
}

check! {
  bool_le_error,
  r#"#!hebi
    true <= false
  "#
}

check! {
  none_le_error,
  r#"#!hebi
    none <= none
  "#
}

check! {
  heterogenous_type_equality,
  r#"#!hebi
  print 1 == "0"
  print 1 == false
  print 1 == true
  print 1 == none
  print none == "string"
  print none == []
  print [] == "string"
  class Test: pass
  print Test() == "string"
  print Test() == none
  print Test() == []
  print to_str == to_int
  print Test() == to_int
  "#
}

check! {
  heterogenous_type_unequality,
  r#"#!hebi
  print 1 != "0"
  print 1 != false
  print 1 != true
  print 1 != none
  print none != "string"
  print none != []
  print [] != "string"
  class Test: pass
  print Test() != "string"
  print Test() != none
  print Test() != []
  print to_str != to_int
  print Test() != to_int
  "#
}

check! {
  object_equality,
  r#"#!hebi
  print to_int == to_int
  print to_str == to_str
  print to_int != to_str
  print [].push != [].push
  x := []
  print x.push == x.push
  print x.is_empty != x.push
  print collect == collect
  print collect != to_int
  print "a" == "a"
  print "a" + "b" == "ab"
  print "a" + "b" != "ba"
  "#
}

check! {
  array_equality,
  r#"#!hebi
    print [] == []
    print [1] == [1]
    print [1, 2] == [1, 2]
    print [1] != []
    print [] != [1]
    print [1, 2] != [2, 1]
    print [1, 2] != ["1", 2]
    print [to_int, to_str] == [to_int, to_str]
  "#
}

check! {
  table_equality,
  r#"#!hebi
    print {} == {}
    print {x: 3} == {x: 3}
    print {x: 3} != {x: none}
    print {x: 3} != {y: 3}
    print {x: 3} != {}
    print {x: 3} != {x: 3, y: 4}
    print {x: [1, 2]} == {x: [1, 2]}
    print {x: [1, 2]} != {x: [2, 3]}
  "#
}

// Reference test for forward and backward jumps <256 instructions
check! {
  small_jump,
  r#"#!hebi
    for i in 0..3:
      x := i + i + i + i + i + i + i + i + i
      print i
  "#
}

// A wide jump test with forward and backwards jumps over >256 instructions
check! {
  wide_jump,
  r#"#!hebi
    for i in 0..3:
      x := i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i
      print i
  "#
}

// NOTE: the constants can be put on a single line once #34 is merged
// Test test pre-allocates a large number of constants, causing more jumps to be
// wide-encoded
check! {
  wide_const_jump,
  r#"#!hebi
    _ = "0"
    _ = "1"
    _ = "2"
    _ = "3"
    _ = "4"
    _ = "5"
    _ = "6"
    _ = "7"
    _ = "8"
    _ = "9"
    _ = "10"
    _ = "11"
    _ = "12"
    _ = "13"
    _ = "14"
    _ = "15"
    _ = "16"
    _ = "17"
    _ = "18"
    _ = "19"
    _ = "20"
    _ = "21"
    _ = "22"
    _ = "23"
    _ = "24"
    _ = "25"
    _ = "26"
    _ = "27"
    _ = "28"
    _ = "29"
    _ = "30"
    _ = "31"
    _ = "32"
    _ = "33"
    _ = "34"
    _ = "35"
    _ = "36"
    _ = "37"
    _ = "38"
    _ = "39"
    _ = "40"
    _ = "41"
    _ = "42"
    _ = "43"
    _ = "44"
    _ = "45"
    _ = "46"
    _ = "47"
    _ = "48"
    _ = "49"
    _ = "50"
    _ = "51"
    _ = "52"
    _ = "53"
    _ = "54"
    _ = "55"
    _ = "56"
    _ = "57"
    _ = "58"
    _ = "59"
    _ = "60"
    _ = "61"
    _ = "62"
    _ = "63"
    _ = "64"
    _ = "65"
    _ = "66"
    _ = "67"
    _ = "68"
    _ = "69"
    _ = "70"
    _ = "71"
    _ = "72"
    _ = "73"
    _ = "74"
    _ = "75"
    _ = "76"
    _ = "77"
    _ = "78"
    _ = "79"
    _ = "80"
    _ = "81"
    _ = "82"
    _ = "83"
    _ = "84"
    _ = "85"
    _ = "86"
    _ = "87"
    _ = "88"
    _ = "89"
    _ = "90"
    _ = "91"
    _ = "92"
    _ = "93"
    _ = "94"
    _ = "95"
    _ = "96"
    _ = "97"
    _ = "98"
    _ = "99"
    _ = "100"
    _ = "101"
    _ = "102"
    _ = "103"
    _ = "104"
    _ = "105"
    _ = "106"
    _ = "107"
    _ = "108"
    _ = "109"
    _ = "110"
    _ = "111"
    _ = "112"
    _ = "113"
    _ = "114"
    _ = "115"
    _ = "116"
    _ = "117"
    _ = "118"
    _ = "119"
    _ = "120"
    _ = "121"
    _ = "122"
    _ = "123"
    _ = "124"
    _ = "125"
    _ = "126"
    _ = "127"
    _ = "128"
    _ = "129"
    _ = "130"
    _ = "131"
    _ = "132"
    _ = "133"
    _ = "134"
    _ = "135"
    _ = "136"
    _ = "137"
    _ = "138"
    _ = "139"
    _ = "140"
    _ = "141"
    _ = "142"
    _ = "143"
    _ = "144"
    _ = "145"
    _ = "146"
    _ = "147"
    _ = "148"
    _ = "149"
    _ = "150"
    _ = "151"
    _ = "152"
    _ = "153"
    _ = "154"
    _ = "155"
    _ = "156"
    _ = "157"
    _ = "158"
    _ = "159"
    _ = "160"
    _ = "161"
    _ = "162"
    _ = "163"
    _ = "164"
    _ = "165"
    _ = "166"
    _ = "167"
    _ = "168"
    _ = "169"
    _ = "170"
    _ = "171"
    _ = "172"
    _ = "173"
    _ = "174"
    _ = "175"
    _ = "176"
    _ = "177"
    _ = "178"
    _ = "179"
    _ = "180"
    _ = "181"
    _ = "182"
    _ = "183"
    _ = "184"
    _ = "185"
    _ = "186"
    _ = "187"
    _ = "188"
    _ = "189"
    _ = "190"
    _ = "191"
    _ = "192"
    _ = "193"
    _ = "194"
    _ = "195"
    _ = "196"
    _ = "197"
    _ = "198"
    _ = "199"
    _ = "200"
    _ = "201"
    _ = "202"
    _ = "203"
    _ = "204"
    _ = "205"
    _ = "206"
    _ = "207"
    _ = "208"
    _ = "209"
    _ = "210"
    _ = "211"
    _ = "212"
    _ = "213"
    _ = "214"
    _ = "215"
    _ = "216"
    _ = "217"
    _ = "218"
    _ = "219"
    _ = "220"
    _ = "221"
    _ = "222"
    _ = "223"
    _ = "224"
    _ = "225"
    _ = "226"
    _ = "227"
    _ = "228"
    _ = "229"
    _ = "230"
    _ = "231"
    _ = "232"
    _ = "233"
    _ = "234"
    _ = "235"
    _ = "236"
    _ = "237"
    _ = "238"
    _ = "239"
    _ = "240"
    _ = "241"
    _ = "242"
    _ = "243"
    _ = "244"
    _ = "245"
    _ = "246"
    _ = "247"
    _ = "248"
    _ = "249"
    _ = "250"
    _ = "251"
    _ = "252"
    _ = "253"
    _ = "254"
    _ = "255"
    for i in 0..3:
      x := i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i + i
      print i
  "#
}
