fn main() {
  use hebi::Hebi;

  let vm = Hebi::default();

  vm.globals().set("greet", vm.create_function(greet));

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
}

#[hebi::function]
fn greet(#[kw] first_name: &str, #[kw] last_name: Option<&str>) {
  if let Some(last_name) = last_name {
    println!("Hello, {first_name} {last_name}!");
  } else {
    println!("Hello, {first_name}!");
  }
}
