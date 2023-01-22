use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use value::Value;

// TODO: turn this into `Isolate`

#[derive(Clone)]
pub struct Context(Arc<Mutex<Inner>>);

struct Inner {
  intern_table: HashMap<String, Value>,
}

impl Context {
  pub fn new() -> Self {
    Self(Arc::new(Mutex::new(Inner {
      intern_table: HashMap::new(),
    })))
  }

  pub fn alloc_string(&self, str: &str) -> Value {
    let mut inner = self.0.lock().unwrap();
    if let Some(str) = inner.intern_table.get(str) {
      str.clone()
    } else {
      let value = Value::from(str);
      inner.intern_table.insert(str.into(), value.clone());
      value
    }
  }
}
