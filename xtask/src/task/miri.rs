use super::common::{cargo, CheckStatus};
use crate::Result;

const MIRIFLAGS: &str = "-Zmiri-tree-borrows -Zmiri-permissive-provenance";

pub fn run(args: &[String]) -> Result<()> {
  cargo("miri")
    .env("MIRIFLAGS", MIRIFLAGS)
    .args(["-F", "serde"])
    .args(args.iter())
    .spawn()?
    .wait()?
    .check()
}
