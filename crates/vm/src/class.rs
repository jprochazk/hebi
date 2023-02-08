use value::object::handle::Handle;
use value::object::{ClassDef, Dict};
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

  if class.borrow().has("init") {
    // call initializer
    let init = unsafe { class.borrow().get("init").cloned().unwrap_unchecked() };
    // FIXME: the initializer never returns back here, it goes straight back to the
    // interpreter there needs to be some way to mark that a `Ret` should
    // actually `Yield` instead of `SwapFrame`
    vm.call(init, args, kwargs)?;
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
