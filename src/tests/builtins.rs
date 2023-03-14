check! {
  types,
  r#"
    print type(100)
    print type(3.14)
    print type(true)
    print type(none)
    print type("test")
  "#
}

check! {
  builtin_methods,
  r#"
    v := []

    v.push(0)
    List.push(v, 1)
    f := List.push
    f(v, 2)
    f := v.push
    f(3)

    print v
  "#
}
