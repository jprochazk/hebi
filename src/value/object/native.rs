#[mu::func]
fn split(str: String, sep: String) -> Vec<String> {
  str.split(&sep).map(|v| v.to_string()).collect()
}
// generates:
struct split__mu_call_impl;
impl mu::Call for split__mu_call_impl {
  fn call(vm: &mut Isolate, func: &Value, args: &[Value], kw: &Value) -> Result<Value, Error> {
    if args.len() < 3 {
      return error!("invalid number of arguments");
    }
    let Some(str) = args[1].as_string() else {
      return error!("invalid type")
    };
    let Some(sep) = args[2].as_string() else {
      return error!("invalid type")
    };
    let result = split(str, sep);
    Ok(Value::from(result))
  }
}

mu::class! {
  // by default, all `pub` fields are exposed, meaning they will receive a generated getter+setter pair.
  // non-`pub` fields are not exposed in any way unless explicitly declared as such.
  //
  // exposed fields must be `From<Value> + Into<Value>`.
  //
  // fields may be configured in the following ways:
  //
  // - `#[mu(readonly)] pub field: T`
  //   effect: only the getter will be generated, and writes to it will fail
  //
  // - `#[mu(private)] pub field: T`
  //   effect: this field will not be exposed
  //
  // - `#[mu(public)] field: T`
  //   effect: this field will be exposed via a generated getter+setter pair.
  //   may be mixed with `readonly`
  //
  struct Counter {
    pub value: i32,
  }

  // all `pub` methods are exposed, as if they had the `#[mu::func]` annotation
  // methods may be configured in the following ways:
  //
  // - `#[mu(private)] pub fn f(...) -> T`
  //   effect: this method will not be exposed
  impl Counter {
    pub fn new(value: i32) -> Counter {
      Counter { value }
    }

    pub fn next(&mut self) -> i32 {
      let temp = self.value;
      self.value += 1;
      temp
    }
  }
}
// generates:
struct counter_new__method;
impl mu::Call for counter_new__method {
  fn call(vm: &mut Isolate, func: &Value, args: &[Value], kw: &Value) -> Result<Value, Error> {
    if args.len() < 2 {
      return error!("invalid number of arguments");
    }
    let Some(class_def) = func.as_native_class_def() else {
      return error!("invalid type")
    };
    let Some(value) = args[0].as_int() else {
      return error!("invalid type")
    };
    let instance = Counter::new(value);
    let instance = class_def.instance(instance);
    Ok(Value::from(instance))
  }
}
struct counter_value__field_get;
impl mu::Call for counter_value__field_get {
  fn call(vm: &mut Isolate, func: &Value, args: &[Value], kw: &Value) -> Result<Value, Error> {
    let Some(instance) = args[0].as_native_class() else {
      return error!("...")
    };
    let instance = instance.inner();
    let value = instance.value.clone();
    Ok(Value::from(value))
  }
}
struct counter_value__field_set;
impl mu::Call for counter_value__field_set {
  fn call(vm: &mut Isolate, func: &Value, args: &[Value], kw: &Value) -> Result<Value, Error> {
    if args.len() < 2 {
      return error!("...");
    }
    let Some(instance) = args[0].as_native_class_mut() else {
      return error!("...")
    };
    let instance = instance.inner_mut();
    let value = args[1].clone();
    // `T` replaced by whatever concrete type is
    let Ok(value) = T::try_from(value) else {
      return error!("...")
    };
    instance.value = value;
    Ok(())
  }
}
struct counter_next__method;
impl mu::Call for counter_next__method {
  fn call(vm: &mut Isolate, func: &Value, args: &[Value], kw: &Value) -> Result<Value, Error> {}
}
impl mu::Class for Counter {
  fn definition() -> ClassDef {
    ClassDef::new()
      .init(counter_new__method)
      .field("value", counter_value__field_get, counter_value__field_set)
      .method("next", counter_next__method)
      .finish()
  }
}

fn test() -> anyhow::Result<()> {
  let mut vm = mu::init();

  let my_module = Module::new()
    .add("Counter", Counter)
    .add("split", split)
    .finish()?;

  vm.register(my_module);

  vm.eval(
    r#"
    for item in split("a b c d", " "):
      print item
    
    counter := Counter(value=0)
  "#,
  )?;

  Ok(())
}
