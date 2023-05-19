use std::ffi::OsStr;
use std::process::{Command, ExitCode, ExitStatus};
use std::{env, fs, path};

fn main() -> ExitCode {
  match try_main() {
    Ok(()) => ExitCode::SUCCESS,
    Err(e) => {
      eprintln!("{e}");
      ExitCode::FAILURE
    }
  }
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

fn try_main() -> Result<()> {
  match env::args().nth(1).as_deref() {
    Some("examples") => examples(),
    _ => {
      print_help();
      Ok(())
    }
  }
}

#[rustfmt::skip]
fn print_help() {
  eprintln!(
"Tasks:
  examples  Run all examples."
  )
}

fn examples() -> Result<()> {
  let examples_dir = project_root().join("examples");

  let mut examples = vec![];

  for example in fs::read_dir(examples_dir)? {
    let example = example?;
    let metadata = example.metadata()?;
    let path = example.path();
    if metadata.is_file() && path.extension() == Some(OsStr::new("rs")) {
      let name = path
        .file_stem()
        .ok_or_else(|| format!("invalid path {}", path.display()))?;
      examples.push(name.to_owned());
    }
  }

  for example in examples {
    cargo("run")
      .arg("--example")
      .arg(&example)
      .spawn()?
      .wait()?
      .check()?;
  }

  Ok(())
}

fn project_root() -> path::PathBuf {
  path::Path::new(&env!("CARGO_MANIFEST_DIR"))
    .ancestors()
    .nth(1)
    .unwrap()
    .to_path_buf()
}

fn cargo(command: impl AsRef<OsStr>) -> Command {
  let mut process = Command::new(env!("CARGO"));
  process.arg(command);
  process
}

trait CheckStatus {
  fn check(&self) -> Result<()>;
}

impl CheckStatus for ExitStatus {
  fn check(&self) -> Result<()> {
    if !self.success() {
      Err(
        format!(
          "Process exited with error code {}",
          self.code().unwrap_or(-1)
        )
        .into(),
      )
    } else {
      Ok(())
    }
  }
}