use std::cell::RefCell;

use hebi::{Hebi, NativeModule};

fn main() {
  struct Circle {
    center: (f64, f64),
    radius: f64,
  }

  impl Circle {
    fn new(center: (f64, f64), radius: f64) -> Self {
      Self { center, radius }
    }

    fn area(&self) -> f64 {
      std::f64::consts::PI * self.radius.powi(2)
    }

    fn unit() -> Circle {
      Circle {
        center: (0.0, 0.0),
        radius: 1.0,
      }
    }
  }

  struct CircleClass(RefCell<Circle>);

  let module = NativeModule::builder("shapes")
    .class::<CircleClass>("Circle", |class| {
      class
        .init(|scope| {
          let radius = scope.param::<f64>(0)?;
          let center = (0.0, 0.0);
          Ok(CircleClass(RefCell::new(Circle::new(center, radius))))
        })
        .field_mut(
          "x",
          |_, this| this.0.borrow().center.0,
          |_, this, value| {
            this.0.borrow_mut().center.0 = value;
            Ok(())
          },
        )
        .field_mut(
          "y",
          |_, this| this.0.borrow().center.1,
          |_, this, value| {
            this.0.borrow_mut().center.1 = value;
            Ok(())
          },
        )
        .method("area", |_, this| this.0.borrow().area())
        .static_method("unit", |scope| {
          scope
            .cx()
            .new_instance(CircleClass(RefCell::new(Circle::unit())))
        })
    })
    .finish();

  let mut hebi = Hebi::new();
  hebi.register(&module);

  hebi
    .eval(
      r#"
from shapes import Circle

c := Circle(20)

print(c.area()) // ~1256
print(3.14 * (c.radius ** 2)) // ~1256
print(Circle.unit().area())
"#,
    )
    .unwrap();
}

/* fn example() {
  struct Foo {
    value: i32,
  }

  impl Foo {
    fn bar(&mut self, f: impl Fn(&mut Self)) {
      f(self)
    }
  }

  let module = NativeModule::builder("test")
    .class::<Foo>("Foo", |class| {
      class.method_mut("bar", |mut scope, this: &mut Foo| {
        let cb = scope.param(0)?;
        this.value = 100;
        scope.call(cb, &[]);
        Ok(())
      });
    })
    .finish();

  let mut hebi = Hebi::new();
  hebi.register(&module);

  hebi
    .eval(
      r#"
from test import Foo

v := Foo()
fn baz():
  fn test():
    print "yo"
  v.bar(test)
v.bar(baz)

# v.bar -> baz -> v.bar -> test
"#,
    )
    .unwrap();
} */
