#[path = "common/common.rs"]
#[macro_use]
mod common;

check! {
  simple_if,
  r#"
    v := 10
    if v > 5:
      print "yes"
    else:
      print "no"
  "#
}
