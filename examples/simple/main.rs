fn main() {
  use hebi::Hebi;

  let vm = Hebi::new();

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
}
