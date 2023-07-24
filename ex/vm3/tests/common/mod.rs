#![allow(dead_code)]

use std::error::Error;
use std::fmt::Display;
use std::fs::{read_dir, read_to_string};
use std::path::{Path, PathBuf};

macro_rules! error {
  ($($tt:tt)*) => {
    SnapshotError { message: format!($($tt)*) }
  }
}

/// Read all files in `input_dir`, building snapshots from their contents using
/// `build_snapshot`, and writing the resulting snapshots to `snapshot_dir`.
///
/// `input_dir` and `snapshot_dir` are relative to the `CARGO_MANIFEST_DIR`.
pub fn snapshot<F>(
  module_name: &str,
  input_dir: impl AsRef<Path>,
  snapshot_dir: impl AsRef<Path>,
  build_snapshot: F,
) -> Result<(), Box<dyn Error>>
where
  F: Fn(&str) -> String,
{
  let input_dir = relative_to_manifest_dir(input_dir);
  let snapshot_dir = relative_to_manifest_dir(snapshot_dir);

  let _settings_scope = use_snapshot_path(snapshot_dir);
  for file in read_dir(input_dir)? {
    let file = file?;
    let path = file.path();
    let name = path
      .file_stem()
      .ok_or_else(|| error!("failed to get file stem for file {}", path.display()))?
      .to_str()
      .ok_or_else(|| {
        error!(
          "failed to convert file stem to string for file {}",
          path.display()
        )
      })?;
    let contents = read_to_string(&path)?;
    eprintln!("building snapshot for {module_name}/{name}");
    let snapshot = build_snapshot(&contents);

    assert_snapshot(module_name, name, &snapshot)?;
  }

  Ok(())
}

#[cfg(not(miri))]
fn use_snapshot_path(path: impl AsRef<Path>) -> insta::internals::SettingsBindDropGuard {
  let mut settings = insta::Settings::clone_current();
  settings.set_snapshot_path(path);
  settings.bind_to_scope()
}

#[cfg(miri)]
fn use_snapshot_path(_: impl AsRef<Path>) {}

// NOTE: We're using insta's internal API which is likely prone to breakage.
// We're fine with taking on that burden in exchange for being able to manage
// how snapshots are laid out.
#[cfg(not(miri))]
fn assert_snapshot(module_name: &str, name: &str, snapshot: &str) -> Result<(), Box<dyn Error>> {
  insta::_macro_support::assert_snapshot(
    insta::_macro_support::ReferenceValue::Named(None),
    snapshot,
    env!("CARGO_MANIFEST_DIR"),
    name,
    module_name,
    file!(),
    line!(),
    "snapshot",
  )
}

#[cfg(miri)]
fn assert_snapshot(_: &str, _: &str, _: &str) -> Result<(), Box<dyn Error>> {
  Ok(())
}

fn relative_to_manifest_dir(path: impl AsRef<Path>) -> PathBuf {
  const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

  Path::new(MANIFEST_DIR).join(path)
}

#[derive(Debug)]
struct SnapshotError {
  message: String,
}

impl Display for SnapshotError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "error while building snapshot: {}", self.message)
  }
}

impl Error for SnapshotError {}
