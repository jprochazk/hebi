fn main() {
  use hebi::Hebi;

  let vm = Hebi::default();

  vm.globals().register_fn("greet", greet);

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
"#,
  )
  .unwrap();

  vm.eval::<()>(r#"greet(first_name="Salman", last_name=none)"#)
    .unwrap();

  vm.globals().register_class::<Test>();
  vm.globals().set("instance", vm.wrap(Test { value: 100 }));

  vm.eval::<()>(
    r#"
print instance.value()
"#,
  )
  .unwrap();

  vm.eval::<()>(
    r#"
print Test.value(instance)
"#,
  )
  .unwrap();
}

#[hebi::function]
fn greet(#[kw] first_name: &str, #[kw] last_name: Option<&str>) {
  if let Some(last_name) = last_name {
    println!("Hello, {first_name} {last_name}!");
  } else {
    println!("Hello, {first_name}!");
  }
}

#[hebi::class]
struct Test {
  value: i32,
}

#[hebi::methods]
impl Test {
  pub fn value(&self) -> i32 {
    self.value
  }
}
