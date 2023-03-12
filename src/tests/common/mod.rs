use std::fmt::Display;

use indexmap::IndexMap;

use crate::ModuleLoader;

pub struct TestModuleLoader {
  pub modules: IndexMap<String, String>,
}
#[derive(Debug)]
pub struct ModuleLoadError {
  name: String,
}
impl Display for ModuleLoadError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "failed to load module `{}`", self.name)
  }
}
impl std::error::Error for ModuleLoadError {}
impl ModuleLoader for TestModuleLoader {
  fn load(&mut self, path: &[String]) -> Result<&str, Box<dyn std::error::Error + 'static>> {
    assert!(path.len() == 1, "cannot import nested path");
    Ok(self.modules.get(path[0].as_str()).ok_or_else(|| {
      Box::new(ModuleLoadError {
        name: path[0].clone(),
      })
    })?)
  }
}

macro_rules! check {
  (
    $name:ident,
    $(modules: {$($module:literal : $code:literal),*},)?
    $(fns: [$($fn_name:ident),*],)?
    $(classes: [$($class_name:ident),*],)?
    $input:literal
  ) => {
    #[test]
    fn $name() {
      use $crate::util::JoinIter;
      use $crate::{Error, Hebi, Value};
      let modules = [
        $($(($module.to_string(), indoc::indoc!($code).to_string()),)*)?
      ].into_iter().collect();
      let module_loader = $crate::tests::common::TestModuleLoader { modules };
      let vm = Hebi::builder()
        .with_io(Vec::new())
        .with_module_loader(module_loader)
        .with_builtins()
        .build();
      $($(
        vm.globals().register_fn(stringify!($fn_name), $fn_name);
      )*)?
      $($(
        vm.globals().register_class::<$class_name>();
      )*)?
      let input = indoc::indoc!($input);
      match vm.eval::<Value>(input) {
        Ok(value) => {
          let stdout = vm.io::<Vec<u8>>().unwrap();
          let stdout = std::str::from_utf8(&stdout[..]).unwrap();
          let modules: &[&str] = &[$($( concat!("# module:", $module, "\n", indoc::indoc!($code), "\n") ),*)?];
          let modules = modules.iter().join("\n");
          let snapshot = format!(
            "\
# Modules:
{modules}

# Input:
{input}

# Result (success):
{value}

# Stdout:
{stdout}
"
          );
          if cfg!(feature = "emit_snapshots") {
            insta::assert_snapshot!(snapshot);
          }
        }
        Err(Error::Syntax(e)) => {
          let mut out = String::new();
          for error in e {
            error.report_to(input, &mut out);
            out.push('\n');
          }
          panic!("parse error:\n{out}")
        }
        Err(Error::Runtime(e)) => {
          panic!("eval error: {}", e)
        }
        Err(Error::Other(e)) => {
          panic!("error: {e}")
        }
      }
    }
  };
}

macro_rules! check_error {
  (
    $name:ident,
    $(modules: {$($module:literal : $code:literal),*},)?
    $(fns: [$($fn_name:ident),*],)?
    $(classes: [$($class_name:ident),*],)?
    $input:literal
  ) => {
    #[test]
    fn $name() {
      use $crate::util::JoinIter;
      use $crate::{Error, Hebi, Value};
      let modules = [
        $($(($module.to_string(), indoc::indoc!($code).to_string()),)*)?
      ].into_iter().collect();
      let module_loader = $crate::tests::common::TestModuleLoader { modules };
      let vm = Hebi::builder()
        .with_io(Vec::new())
        .with_module_loader(module_loader)
        .with_builtins()
        .build();
      $($(
        vm.globals().register_fn(stringify!($fn_name), $fn_name);
      )*)?
      $($(
        vm.globals().register_class::<$class_name>();
      )*)?
      let input = indoc::indoc!($input);
      match vm.eval::<Value>(input) {
        Ok(value) => {
          let stdout = String::from_utf8(vm.io::<Vec<u8>>().unwrap().clone()).unwrap();
          panic!("unexpected eval success, value={value}, stdout=`{stdout:?}`")
        }
        Err(Error::Syntax(e)) => {
          let mut out = String::new();
          for error in e {
            error.report_to(input, &mut out);
            out.push('\n');
          }
          panic!("failed to parse module:\n{out}")
        }
        Err(Error::Other(e)) => {
          panic!("unexpected error: {e}")
        }
        Err(Error::Runtime(error)) => {
          let stdout = vm.io::<Vec<u8>>().unwrap();
          let stdout = std::str::from_utf8(&stdout[..]).unwrap();
          let modules: &[&str] = &[$($( concat!("# module:", $module, "\n", $code, "\n") ),*)?];
          let modules = modules.iter().join("\n");
          let snapshot = format!(
            "\
# Modules:
{modules}

# Input:
{input}

# Result (error):
{error}

# Stdout:
{stdout}
"
          );
          if cfg!(feature = "emit_snapshots") {
            insta::assert_snapshot!(snapshot);
          }
        }
      }
    }
  };
}
