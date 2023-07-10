use clap::Parser;
use hebi_cli::commands::Command;
use hebi_cli::common::InputArgs;

#[derive(Debug, Parser)]
#[clap(name = "hebi", version)]
pub struct App {
  #[clap(subcommand)]
  command: Option<Command>,

  // Args for the default run command
  #[clap(flatten)]
  input: InputArgs,
}

fn main() -> anyhow::Result<()> {
  let app = App::parse();

  let command = app
    .command
    .unwrap_or_else(|| Command::run(app.input.clone()));

  command.execute()?;

  Ok(())
}
