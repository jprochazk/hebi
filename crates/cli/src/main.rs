mod repl;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
  #[command(subcommand)]
  cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
  Repl,
}

fn main() -> anyhow::Result<()> {
  let args = Cli::parse();
  match args.cmd {
    Some(Cmd::Repl) | None => Ok(repl::run()?),
    _ => Ok(()),
  }
}
