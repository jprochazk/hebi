fn main() {
  use hebi::Hebi;

  let vm = Hebi::default();

  vm.globals()
    .set("my_bound_fn", vm.create_function(my_bound_fn));

  println!("{}", vm.eval::<i32>("1 + 1").unwrap());

  vm.eval::<()>(
    r#"
class Test:
  v = 10
  fn test(self):
    print self.v

t := Test(v=100)
t.test() # prints 100
t.v = 20
t.test() # prints 20

my_bound_fn(t)
"#,
  )
  .unwrap();
}

#[hebi::function]
fn my_bound_fn(v: hebi::Value) -> hebi::Value {
  println!("got: {v}");
  v
}
