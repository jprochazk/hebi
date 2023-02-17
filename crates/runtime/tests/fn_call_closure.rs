#[path = "common/common.rs"]
#[macro_use]
mod common;

check! {
  simple,
  r#"
    fn a():
      v := 10
      fn b():
        print v
      return b

    a()()
  "#
}
check! {
  counter,
  r#"
    fn counter(start=0, *, step=1):
      class State:
        value = start
      state := State()
      fn inner():
        temp := state.value
        state.value += step
        return temp
      return inner
    
    c := counter()
    print c()
    print c()
    print c()
  "#
}
check! {
  nested,
  r#"
    fn a():
      fn b():
        v := 10
        fn c():
          fn d():
            return v
          return d
        return c
      return b
    
    print a()()()()
  "#
}
