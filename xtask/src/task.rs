use crate::Result;

const HELP: &str = "
Usage:
  xtask <task> <args>

Tasks:
  examples : run all examples
  snap     : run snapshot tests in review mode
  miri     : run cargo command under miri
  test-all : run tests and examples
";

pub mod common;

pub mod bench;
pub mod examples;
pub mod miri;
pub mod snap;
pub mod template;
pub mod test;

pub fn print_help() -> Result<()> {
  eprintln!("{HELP}");
  Ok(())
}

pub fn run(which: &str, args: &[String]) -> Result<()> {
  match which {
    "examples" => examples::run(args),
    "snap" => snap::run(args),
    "miri" => miri::run(args),
    "test" => test::run(args),
    "bench" => bench::run(args),
    // "template" => template::run(args),
    _ => print_help(),
  }
}
