check! {
  meta_binop,
  r#"
    class T:
      value = 0
      fn meta:add(self, other):
        self.value += other.value
        return self

    a := T(value=2)
    b := T(value=5)
    r := a + b
    print r.value
  "#
}
