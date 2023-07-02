use super::common::{cargo, CheckStatus};
use crate::Result;

pub fn run(args: &[String]) -> Result<()> {
  cargo("bench")
    .env("RUSTFLAGS", "--cfg enable_slow_bench")
    .args(args)
    .spawn()?
    .wait()?
    .check()?;

  Ok(())
}
