use std::ffi::OsStr;
use std::fs;

use super::common::{cargo, project_root, CheckStatus};
use crate::Result;

pub fn run(args: &[String]) -> Result<()> {
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
      .arg("--all-features")
      .args(args.iter())
      .spawn()?
      .wait()?
      .check()?;
  }

  Ok(())
}
