use super::common::{cargo, CheckStatus};
use crate::Result;

pub fn run(args: &[String]) -> Result<()> {
  cargo("test")
    .arg("--all-targets")
    .spawn()?
    .wait()?
    .check()?;

  super::examples::run(args)?;

  Ok(())
}
