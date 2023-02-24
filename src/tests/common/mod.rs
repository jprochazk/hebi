#[macro_export]
macro_rules! check {
  ($name:ident, $input:literal) => {
    #[test]
    fn $name() {
      use $crate::{EvalError, Hebi, Value};
      let input = indoc::indoc!($input);
      let vm = Hebi::with_io(Vec::new());
      match vm.eval::<Value>(input) {
        Ok(value) => {
          let stdout = vm.io::<Vec<u8>>().unwrap();
          let stdout = std::str::from_utf8(&stdout[..]).unwrap();
          let snapshot = format!(
            "\
# Input:
{input}

# Result (success):
{value}

# Stdout:
{stdout}
"
          );
          insta::assert_snapshot!(snapshot);
        }
        Err(EvalError::Parse(errors)) => {
          let mut out = String::new();
          for error in errors {
            error.report_to(input, &mut out);
            out.push('\n');
          }
          panic!("parse error:\n{out}")
        }
        Err(EvalError::Runtime(e)) => {
          panic!("eval error:\n{}", e.stack_trace(Some(input.into())))
        }
      }
    }
  };
}

#[macro_export]
macro_rules! check_error {
  ($name:ident, $input:literal) => {
    #[test]
    fn $name() {
      use $crate::{EvalError, Hebi, Value};
      let input = indoc::indoc!($input);
      let vm = Hebi::with_io(Vec::new());
      match vm.eval::<Value>(input) {
        Ok(value) => {
          let stdout = String::from_utf8(vm.io::<Vec<u8>>().unwrap().clone()).unwrap();
          panic!("unexpected eval success, value={value}, stdout=`{stdout:?}`")
        }
        Err(EvalError::Parse(errors)) => {
          let mut out = String::new();
          for error in errors {
            error.report_to(input, &mut out);
            out.push('\n');
          }
          panic!("failed to parse module:\n{out}")
        }
        Err(EvalError::Runtime(error)) => {
          let error = error.stack_trace(Some(input.into()));
          let stdout = vm.io::<Vec<u8>>().unwrap();
          let stdout = std::str::from_utf8(&stdout[..]).unwrap();
          let snapshot = format!(
            "\
# Input:
{input}

# Result (error):
{error}

# Stdout:
{stdout}
"
          );
          insta::assert_snapshot!(snapshot);
        }
      }
    }
  };
}
