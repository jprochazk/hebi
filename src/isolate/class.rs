use super::{call, Isolate};
use crate::value::handle::Handle;
use crate::value::object::{Class, Key, Method};
use crate::value::Value;
use crate::Result;

impl Isolate {
  pub fn create_instance(
    &mut self,
    def: Handle<Class>,
    args: &[Value],
    kwargs: Value,
  ) -> Result<Value> {
    // create instance
    let mut class = self.ctx.alloc(def.instance());

    if class.has("init") {
      let init = class.get(Key::Ref("init")).unwrap().clone();
      // call initializer
      // TODO: don't allocate temp object here
      self.call(
        Value::object(self.alloc(Method::new(Value::object(class.clone()), init))),
        args,
        kwargs,
      )?;
    } else {
      // assign kwargs to fields
      if let Some(kwargs) = kwargs.to_dict() {
        call::check_args(true, def.params(), args, Some(kwargs.clone()))?;
        for (k, v) in kwargs.iter() {
          class.insert(k.clone(), v.clone());
        }
      } else {
        call::check_args(true, def.params(), args, None)?;
      }
    }
    class.freeze();

    Ok(Value::object(class))
  }
}
