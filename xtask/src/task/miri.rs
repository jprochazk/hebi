use super::common::{cargo, CheckStatus};
use crate::Result;

const MIRIFLAGS: &str = "-Zmiri-tree-borrows -Zmiri-permissive-provenance";

pub fn run(args: &[String]) -> Result<()> {
  cargo("miri")
    .env("MIRIFLAGS", MIRIFLAGS)
    .args(args.iter())
    .args(["--no-default-features", "-F", "nanbox", "-F", "serde"])
    .args(["--"])
    .args(["serde", "util", "value", "vm", "ptr"])
    .spawn()?
    .wait()?
    .check()
}
