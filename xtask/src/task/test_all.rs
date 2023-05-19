use super::common::{cargo, CheckStatus};
use crate::Result;

pub fn run(args: &[String]) -> Result<()> {
  cargo("test")
    .arg("--all-targets")
    .args(["-F", "serde"])
    .spawn()?
    .wait()?
    .check()?;

  super::examples::run(args)?;

  Ok(())
}
