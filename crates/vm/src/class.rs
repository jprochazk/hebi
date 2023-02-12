use value::object::handle::Handle;
use value::object::{ClassDef, Dict, Method};
use value::Value;

use crate::{call, Error, Isolate};

pub fn create_instance<Io: std::io::Write>(
  vm: &mut Isolate<Io>,
  def: Handle<ClassDef>,
  args: &[Value],
  kwargs: Value,
) -> Result<Value, Error> {
  // create instance
  let mut class = def.borrow().instance();

  if class.borrow().get("init").is_some() {
    let init = class.borrow().get("init").unwrap().clone();
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
      call::check_args(true, def.borrow().params(), args, &kwargs)?;
      let mut class = class.borrow_mut();
      for (k, v) in kwargs.iter() {
        class.insert(k.clone(), v.clone());
      }
    } else {
      call::check_args(true, def.borrow().params(), args, &Dict::new())?;
    }
  }

  class.borrow_mut().freeze();
  Ok(class.widen().into())
}
