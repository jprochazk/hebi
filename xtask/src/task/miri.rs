use super::common::{cargo, CheckStatus};
use crate::Result;

const MIRIFLAGS: &str = "-Zmiri-tree-borrows -Zmiri-permissive-provenance";

const ALL: &[&str] = &["serde", "util", "value", "vm", "ptr"];

pub fn print_help() -> Result<()> {
  eprintln!(
    r#"
Usage:
  xtask miri <command> [filters..]

Options:
  command : `test` or `run`
  filters : test filter - works the same way as in `cargo test [filters..]`.
            if empty, it will run a specific subset of tests which
            contains sensitive unsafe code.
"#
  );

  Ok(())
}

pub fn run(args: &[String]) -> Result<()> {
  let cmd = match args.get(0) {
    Some(cmd) => cmd,
    None => return print_help(),
  };
  let filters = &args[1..];

  match cmd.as_str() {
    "test" => miri("test", filters),
    "run" => miri("run", filters),
    _ => print_help(),
  }
}

fn should_run_all(filter: &[String]) -> bool {
  filter.is_empty()
}

fn miri(cmd: &str, filters: &[String]) -> Result<()> {
  let mut process = cargo("miri");

  process
    .env("MIRIFLAGS", MIRIFLAGS)
    .arg(cmd)
    .args(["--no-default-features", "-F", "nanbox", "-F", "serde"])
    .args(["--"]);

  if should_run_all(filters) {
    process.args(ALL);
  } else {
    process.args(filters);
  }

  process.spawn()?.wait()?.check()
}
