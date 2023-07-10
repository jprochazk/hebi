use std::io::Read;
use std::path::PathBuf;

use anyhow::Context;
use clap::builder::{PathBufValueParser, TypedValueParser};
use clap::Args;

#[derive(Clone, Debug, Args)]
pub struct InputArgs {
  /// The path to the Hebi file to process. Required unless code is supplied
  /// through stdin.
  #[arg(
    value_name = "FILE", 
    value_parser = PathBufValueParser::new().map(PathOrStdin::new),
    default_value = "-", 
    hide_default_value = true
  )]
  file: PathOrStdin,
}

impl InputArgs {
  /// Returns the source code of the input script.
  pub fn source(&self) -> anyhow::Result<String> {
    self.file.read()
  }

  /// Returns the name of the input script
  pub fn name(&self) -> std::borrow::Cow<'static, str> {
    match &self.file {
      PathOrStdin::Path(p) => p.display().to_string().into(),
      PathOrStdin::NonTtyStdin => "script".into(),
    }
  }
}

/// Either a path to a file or a potential stdin stream
#[derive(Clone, Debug)]
pub enum PathOrStdin {
  /// Specifies the path to the script to process.
  Path(PathBuf),
  /// Indicates that the script will be supplied through stdin (for example, by
  /// piping it into the program). If stdin is a TTY, this option will fail.
  NonTtyStdin,
}

impl PathOrStdin {
  /// Creates a new [`PathOrStdin`] from a [`PathBuf`]. If the path is `-`, then
  /// [`PathOrStdin::NonTtyStdin`] will be used.
  pub fn new(maybe_path: PathBuf) -> Self {
    if maybe_path.as_os_str() == "-" {
      Self::NonTtyStdin
    } else {
      Self::Path(maybe_path)
    }
  }

  /// Attempts to read the script either from disk or from stdin.
  /// Fails if the file doesn't exist or if stdin is a TTY.
  pub fn read(&self) -> anyhow::Result<String> {
    Ok(match self {
      PathOrStdin::Path(p) => std::fs::read_to_string(p)
        .with_context(|| format!("Failed to read file at {}", p.display()))?,
      PathOrStdin::NonTtyStdin => {
        if atty::isnt(atty::Stream::Stdin) {
          let mut buf = String::new();
          std::io::stdin()
            .read_to_string(&mut buf)
            .with_context(|| "Failed to read from stdin")?;
          buf
        } else {
          anyhow::bail!("An input file is required")
        }
      }
    })
  }
}
