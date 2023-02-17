use super::error::Error;
use super::{call, Isolate};
use crate::value::object::handle::Handle;
use crate::value::object::{ClassDef, Dict, Method};
use crate::value::Value;

pub fn create_instance<Io: std::io::Write>(
  vm: &mut Isolate<Io>,
  def: Handle<ClassDef>,
  args: &[Value],
  kwargs: Value,
) -> Result<Value, Error> {
  // create instance
  let mut class = def.instance();

  if class.get("init").is_some() {
    let init = class.get("init").unwrap().clone();
    // call initializer
    // TODO: don't allocate temp object here
    vm.call(
      Method {
        this: class.clone().into(),
        func: init,
      }
      .into(),
      args,
      kwargs,
    )?;
  } else {
    // assign kwargs to fields
    if let Some(kwargs) = kwargs.as_dict() {
      call::check_args(true, def.params(), args, kwargs)?;
      for (k, v) in kwargs.iter() {
        class.insert(k.clone(), v.clone());
      }
    } else {
      call::check_args(true, def.params(), args, &Dict::new())?;
    }
  }

  class.freeze();
  Ok(class.widen().into())
}
