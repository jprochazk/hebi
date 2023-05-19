use super::common::{cargo, CheckStatus};
use crate::Result;

pub fn run(args: &[String]) -> Result<()> {
  cargo("insta")
    .args([
      "test",
      "--review",
      "--delete-unreferenced-snapshots",
      "--no-ignore",
    ])
    .args(args.iter())
    .spawn()?
    .wait()?
    .check()
}