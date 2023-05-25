use hebi::{Hebi, NativeModule};

fn main() {
  // @dataclass
  // you can read the data
  // you can't mutate it
  // you can't construct an instance of `Account`
  /* #[derive(Data)]
  struct Account {
    trust_level: i32,
  }

  let module = NativeModule::builder("types")
    .class("Account", Account)
    .finish(); */

  let mut hebi = Hebi::new();
  // hebi.register(&module);

  /* hebi
  .global()
  .set("user", hebi.new_instance(Account { trust_level: 0 })); */

  hebi
    .eval(
      r#"
if user.trust_level > 100:
  print "trusted"
else:
  print "untrusted"
"#,
    )
    .unwrap()
    .as_bool();
}
