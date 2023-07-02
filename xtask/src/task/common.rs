use std::ffi::OsStr;
use std::process::{Command, ExitStatus};
use std::{env, path};

use crate::Result;

pub fn project_root() -> path::PathBuf {
  path::Path::new(&env!("CARGO_MANIFEST_DIR"))
    .ancestors()
    .nth(1)
    .unwrap()
    .to_path_buf()
}

pub fn cargo(command: impl AsRef<OsStr>) -> Command {
  let mut process = Command::new(env!("CARGO"));
  process.arg(command);
  process
}

pub trait CheckStatus {
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
