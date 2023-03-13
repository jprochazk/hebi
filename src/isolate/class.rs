use super::call::Args;
use super::{call, Isolate};
use crate::value::handle::Handle;
use crate::value::object::{native, Class, NativeClass, NativeClassInstance};
use crate::value::Value;
use crate::{Error, Result};

// TODO: `kwargs: Value` -> `kwargs: Option<Handle<Dict>>`

impl Isolate {
  pub fn create_instance(&mut self, class: Handle<Class>, args: Args) -> Result<Value> {
    // create instance
    let mut instance = self.ctx.alloc(class.instance());

    if let Some(init) = class.init() {
      // call initializer
      self.call_recurse(init, args.with_receiver(instance.clone().into()))?;
    } else {
      call::check_args(true, class.params(), &args)?;
      // assign kwargs to fields
      if let Some(kw) = args.all_kw() {
        for (k, v) in kw.iter() {
          instance.insert(k.clone(), v.clone());
        }
      }
    }
    instance.freeze();

    Ok(Value::object(instance))
  }

  // TODO: add a way to create native instances from native functions

  pub fn create_native_instance(
    &mut self,
    class: Handle<NativeClass>,
    args: Args,
  ) -> Result<Value> {
    let Some(init) = class.init() else {
      return Err(Error::runtime(format!("cannot initialize class {}", class.name())))
    };

    let user_data = native::call_native_fn(
      init,
      &self.ctx,
      args.this().clone(),
      unsafe { args.all_pos() },
      args.all_kw(),
    )?
    .to_user_data()
    .expect("init function returned something other than UserData");

    let instance = NativeClassInstance::new(&self.ctx, class, user_data);

    Ok(instance.into())
  }
}
