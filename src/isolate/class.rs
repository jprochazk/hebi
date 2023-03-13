use super::{call, Isolate};
use crate::value::handle::Handle;
use crate::value::object::native::Function;
use crate::value::object::{Class, Method, NativeClass, NativeClassInstance};
use crate::value::Value;
use crate::{public, Error, Result};

// TODO: `kwargs: Value` -> `kwargs: Option<Handle<Dict>>`

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
      let init = class.get("init").unwrap().clone();
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

  // TODO: add a way to create native instances from native functions

  pub fn create_native_instance(
    &mut self,
    class: Handle<NativeClass>,
    args: &[Value],
    kwargs: Value,
  ) -> Result<Value> {
    let Some(init) = class.init() else {
      return Err(Error::runtime(format!("cannot initialize class {}", class.name())))
    };

    let user_data = Function::call(
      &init,
      &public::Context::bind(self.ctx()),
      public::Value::none(),
      public::Value::bind_slice(args),
      kwargs.to_dict().map(public::Dict::bind),
    )?
    .unbind()
    .to_user_data()
    .expect("init function returned something other than UserData");

    let instance = NativeClassInstance::new(&self.ctx, class, user_data);

    Ok(instance.into())
  }
}
