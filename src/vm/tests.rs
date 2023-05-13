#[macro_use]
mod macros;

use std::collections::HashMap;

use super::*;
use crate as hebi;

check! {
  example,
  r#"
    v := 0
    v
  "#
}

check! {
  make_fn,
  r#"
    fn test():
      return "yo"

    test
  "#
}

check! {
  basic_fn_call,
  r#"
    fn test():
      return "yo"

    test()
  "#
}

check! {
  closure_call,
  r#"
    fn outer():
      v := "yo"
      fn inner():
        return v
      return inner
    
    outer()()
  "#
}

check! {
  make_fn_with_args,
  r#"
    fn test(v):
      return v
    
    test
  "#
}

check! {
  call_fn_with_args,
  r#"
    fn test(v):
      return v
    
    test(100)
  "#
}

check! {
  call_fn_with_args__error_not_enough_args,
  r#"
    fn test(v):
      return v
    
    test()
  "#
}

check! {
  call_fn_with_args__error_too_many_args,
  r#"
    fn test(v):
      return v
    
    test(100, 100)
  "#
}

check! {
  call_fn_recursive,
  r#"
    fn test(v):
      if v: return "yo"
      else: return test(true)
    
    test(true)
    test(false)
  "#
}

struct TestModuleLoader {
  modules: HashMap<&'static str, &'static str>,
}

impl TestModuleLoader {
  pub fn new(modules: &[(&'static str, &'static str)]) -> Self {
    Self {
      modules: HashMap::from_iter(modules.iter().cloned()),
    }
  }
}

impl module::Loader for TestModuleLoader {
  fn load(&self, path: &str) -> hebi::Result<&str> {
    match self.modules.get(path).copied() {
      Some(module) => Ok(module),
      None => Err(Error::Vm(SpannedError::new(
        format!("module `{path}` not found"),
        None,
      ))),
    }
  }
}

check! {
  module
  import_value,
  {
    test: "value := 100"
  },
  r#"
    import test
    test.value
  "#
}

check! {
  module
  import_value_named,
  {
    test: "value := 100"
  },
  r#"
    from test import value
    value
  "#
}

check! {
  module
  import_fn,
  {
    test: r#"
      fn test(value):
        return value
    "#
  },
  r#"
    import test
    test.test("yo")
  "#
}

check! {
  module
  import_fn_named,
  {
    test: r#"
      fn test(value):
        return value
    "#
  },
  r#"
    from test import test
    test("yo")
  "#
}

check! {
  module
  module_vars,
  {
    test: r#"
      value := 100
      fn set(v):
        value = v
      fn get():
        return value
    "#
  },
  r#"
    import test
    
    test.set(50)
    test.get()
  "#
}

check! {
  module
  module_vars_per_module,
  {
    test: r#"
      value := 100
      fn set(v):
        value = v
      fn get():
        return value
    "#
  },
  r#"
    import test

    value := 200
    test.set(50)
    value = 0
    test.get()
  "#
}

check! {
  module
  module_fail_to_parse,
  {
    test: r#"
      fn invalid:
        pass
    "#
  },
  r#"
    import test
  "#
}

check! {
  module
  module_not_found,
  {},
  r#"
    import test
  "#
}
