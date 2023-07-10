use hebi::{Hebi, NativeModule, Scope};

pub fn build_hebi() -> Hebi {
  let mut hebi = Hebi::new();
  hebi.register(&self::io::build());
  hebi.register(&self::parsing::build());
  hebi
}

pub fn report_errors(source: &str, e: hebi::Error) {
  let color = supports_color::on(supports_color::Stream::Stderr)
    .map(|c| c.has_basic)
    .unwrap_or(false);
  eprintln!("{}", e.report(source, color));
}

mod parsing {
  use hebi::{IntoValue, Value};

  use super::*;

  pub fn build() -> hebi::NativeModule {
    NativeModule::builder("parsing")
      .function("int", parse_int)
      .function("float", parse_float)
      .finish()
  }

  fn parse_int(scope: Scope<'_>) -> hebi::Result<Value> {
    let value = scope.param::<String>(0)?;
    let result = value
      .as_str()
      .parse::<i32>()
      .map(|n| n.into_value(scope.global()))
      .unwrap_or_else(|_| None::<i32>.into_value(scope.global()));

    result
  }

  fn parse_float(scope: Scope<'_>) -> hebi::Result<Value> {
    let value = scope.param::<String>(0)?;
    let result = value
      .as_str()
      .parse::<f64>()
      .map(|n| n.into_value(scope.global()))
      .unwrap_or_else(|_| None::<f64>.into_value(scope.global()));

    result
  }
}

mod io {
  use std::io::Write;

  use super::*;

  pub fn build() -> hebi::NativeModule {
    NativeModule::builder("io")
      .function("input", hebi_input)
      .finish()
  }

  fn hebi_input(scope: Scope<'_>) -> hebi::Result<String> {
    let prompt = scope.param::<String>(0)?;

    let mut stdout = std::io::stdout();

    write!(stdout, "{}", prompt).map_err(|_| hebi::error!("Failed to write prompt"))?;
    stdout
      .flush()
      .map_err(|_| hebi::error!("Failed to flush stdout"))?;

    let mut buf = String::new();

    let bytes_read = std::io::stdin()
      .read_line(&mut buf)
      .map_err(|_| hebi::error!("Failed to read input"))?;

    if bytes_read == 0 {
      return Err(hebi::error!("EOF error while reading input").into());
    }

    while buf.ends_with(&['\n', '\r']) {
      buf.pop();
    }

    Ok(buf)
  }
}
