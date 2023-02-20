use super::{call, Isolate};
use crate::value::object::handle::Handle;
use crate::value::object::{ClassDef, Method};
use crate::value::Value;
use crate::Result;

pub fn create_instance<Io: std::io::Write>(
  vm: &mut Isolate<Io>,
  def: Handle<ClassDef>,
  args: &[Value],
  kwargs: Value,
) -> Result<Value> {
  // create instance
  let mut class = Handle::alloc(def.instance());

  if class.has("init") {
    let init = class.get("init").unwrap().clone();
    // call initializer
    // TODO: don't allocate temp object here
    vm.call(
      Value::object(Handle::alloc(Method::new(
        Value::object(class.clone()),
        init,
      ))),
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
