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

    class Counter:
      fn init(self, max = [a,b,c], other = d):
        self.n = 0
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

fn f((a,): (A,)) {
  true
}
