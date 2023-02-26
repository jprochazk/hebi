check! {
  simple_load,
  modules: {
    "test": "value := 100"
  },
  r#"
    import test

    print test.value
  "#
}
check! {
  simple_load_named,
  modules: {
    "test": "value := 100"
  },
  r#"
    from test import value

    print value
  "#
}
check! {
  load_fn,
  modules: {
    "test": r#"
      fn f(value):
        print value
    "#
  },
  r#"
    import test

    print test.f(100)
  "#
}
check! {
  load_fn_named,
  modules: {
    "test": r#"
      fn f(value):
        print value
    "#
  },
  r#"
    from test import f

    print f(100)
  "#
}
check! {
  load_fn_with_module_vars,
  modules: {
    "test": r#"
      value := 100
      fn set(v):
        value = v
      fn get():
        return value
    "#
  },
  r#"
    from test import get, set

    print get()
    set(0)
    print get()
  "#
}
check! {
  load_class,
  modules: {
    "test": r#"
      class Test:
        v = 100
    "#
  },
  r#"
    from test import Test

    t := Test()
    print t.v
  "#
}
check! {
  load_class_with_module_vars,
  modules: {
    "test": r#"
      value := 100
      class Test:
        fn f(self):
          return value
    "#
  },
  r#"
    from test import Test

    t := Test()
    print t.f()
  "#
}
check! {
  load_fn_with_captures_and_module_vars,
  modules: {
    "test": r#"
      step := 2
      fn make_counter(start):
        class State:
          value = start
        state := State()
        fn inner():
          temp := state.value
          state.value += step
          return temp
        return inner
    "#
  },
  r#"
    from test import make_counter

    c := make_counter(10)
    print c()
    print c()
    print c()

  "#
}
check! {
  multi_module_nesting,
  modules: {
    "nested": r#"
      bar := 100
    "#,
    "test": r#"
      from nested import bar

      foo := bar
    "#
  },
  r#"
    from test import foo

    print foo
  "#
}
check! {
  multi_module_nesting_mut,
  modules: {
    "nested": r#"
      bar := 100
    "#,
    "test": r#"
      from nested import bar

      foo := bar
    "#
  },
  r#"
    import nested

    nested.bar = 200

    from test import foo

    print foo
  "#
}
