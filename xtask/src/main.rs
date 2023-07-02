mod task;

use std::env;
use std::process::ExitCode;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

fn try_main() -> Result<()> {
  let mut args = env::args();
  let task = args.nth(1);
  let args = args.collect::<Vec<_>>();
  match task.as_deref() {
    Some(task) => task::run(task, &args),
    None => task::print_help(),
  }
}

fn main() -> ExitCode {
  match try_main() {
    Ok(()) => ExitCode::SUCCESS,
    Err(e) => {
      eprintln!("{e}");
      ExitCode::FAILURE
    }
  }
}
