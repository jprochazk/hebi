use std::cell::RefCell;
use std::rc::Rc;

use beef::lean::Cow;
use indexmap::IndexMap;

use crate::object::ptr::Ptr;
use crate::object::String;

#[derive(Default, Clone)]
pub struct Context {
  inner: Rc<RefCell<Inner>>,
}

#[derive(Default)]
struct Inner {
  // TODO: try using one large table to store all the strings
  string_table: IndexMap<Cow<'static, str>, Ptr<String>>,
}

impl Context {
  pub fn intern(&self, s: impl Into<Cow<'static, str>>) -> Ptr<String> {
    let s = s.into();

    if let Some(s) = self.inner.borrow().string_table.get(&s) {
      return s.clone();
    }

    let v = self.alloc(String::new(s.clone()));
    self.inner.borrow_mut().string_table.insert(s, v.clone());
    v
  }
}

impl Context {
  #[cfg(test)]
  pub(crate) fn for_test() -> Context {
    Context::default()
  }
}
