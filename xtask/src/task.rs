use crate::Result;

pub mod common;

pub mod examples;
pub mod miri;
pub mod snap;
pub mod test_all;

pub fn run(which: &str, args: &[String]) -> Result<()> {
  match which {
    "examples" => examples::run(args),
    "snap" => snap::run(args),
    "miri" => miri::run(args),
    "test-all" => test_all::run(args),
    _ => print_help(),
  }
}

const HELP: &str = "\
Tasks:
  examples : run all examples
  snap     : run snapshot tests in review mode
  miri     : run cargo command under miri
  test-all : run tests and examples
";

pub fn print_help() -> Result<()> {
  eprint!("{HELP}");
  Ok(())
}
