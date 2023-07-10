use clap::{Args, Subcommand};

use crate::common::InputArgs;

#[derive(Clone, Debug, Subcommand)]
pub enum Command {
  /// Execute a Hebi file [default]
  Run(RunArgs),
  /// Disassemble a Hebi file.
  #[clap(visible_alias = "dis")]
  Disassemble(InputArgs),
  Repl,
}
impl Command {
  pub fn run(input: InputArgs) -> Self {
    Self::Run(RunArgs { input, dump: false })
  }

  pub fn execute(self) -> anyhow::Result<()> {
    match self {
      Self::Run(args) => handle_run(args),
      Self::Disassemble(args) => handle_disassemble(args),
      Self::Repl => handle_repl(),
    }
  }
}

#[derive(Clone, Debug, Args)]
pub struct RunArgs {
  /// If provided, dumps the VM state after execution.
  #[clap(long, default_value_t = false)]
  dump: bool,
  #[clap(flatten)]
  input: InputArgs,
}

fn handle_repl() -> anyhow::Result<()> {
  crate::repl::run().map_err(|e| anyhow::anyhow!(e))?;
  Ok(())
}

fn handle_run(args: RunArgs) -> anyhow::Result<()> {
  let source = args.input.source()?;

  let mut hebi = crate::hebi::build_hebi();
  match hebi.eval(&source) {
    Ok(_) => {
      if args.dump {
        eprintln!("{:#?}", hebi);
      }
    }
    Err(e) => {
      if args.dump {
        eprintln!("{:#?}", hebi);
      }
      crate::hebi::report_errors(&source, e);
      anyhow::bail!("Failed to run {}", args.input.name());
    }
  }
  Ok(())
}

fn handle_disassemble(input: InputArgs) -> anyhow::Result<()> {
  let source = input.source()?;

  let hebi = crate::hebi::build_hebi();
  let chunk = match hebi.compile(&source) {
    Ok(chunk) => chunk,
    Err(e) => {
      crate::hebi::report_errors(&source, e);
      anyhow::bail!("Failed to disassemble {}", input.name());
    }
  };

  println!("{}", chunk.disassemble());

  Ok(())
}
