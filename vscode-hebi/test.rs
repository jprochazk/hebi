check! {
  test,
  r#"#!hebi
    # statements

    pass
    return
    return v
    continue
    break
    yield
    yield v
    print a, b
    a := v
    a = v
    a += v
    a -= v
    a *= v
    a /= v
    a %= v
    a **= v
    a ??= v

    import a as b
    from a import b
    from a import b, c, d

    if true: pass
    for i in start..end: pass
    while true: pass
    loop: pass

    fn a(): pass

    class T: pass

    class T(U): pass

    class T:
      v = 0
      fn f(): pass

    none true false

    self super

    +a
    -a
    !a
    ?a

    a(b)
    a[b]
    a.b

    var

    _100_000
    100_000
    10.0
    "asdf"
    [a,b,c]
    {a:b, ["c"]:d}

    (expr)
  "#
}
