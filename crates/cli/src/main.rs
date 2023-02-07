use rustyline::Editor;
use value::object::Registry;
use value::Value;
use vm::Isolate;

struct Repl {
  emit_ctx: emit::Context,
  vm: Isolate,
  editor: Editor<()>,
}

enum ParseResult {
  Incomplete,
  Complete,
}

enum Error {
  Readline(rustyline::error::ReadlineError),
  Parse(String),
}

impl Repl {
  fn new() -> Self {
    Self {
      emit_ctx: emit::Context::new(),
      vm: Isolate::new(Registry::new().into()),
      editor: Editor::new().unwrap(),
    }
  }

  fn read_multi_line_input(&mut self, buffer: &mut String) -> Result<(), Error> {
    // first line
    buffer.clear();
    loop {
      buffer.push('\n');
      let line = self.editor.readline("> ").map_err(Error::Readline)?;
      self.editor.add_history_entry(&line);
      buffer.push_str(&line);

      match self.validate(buffer.as_str()).map_err(Error::Parse)? {
        ParseResult::Incomplete => continue,
        ParseResult::Complete => break Ok(()),
      }
    }
  }

  fn eval(&mut self, input: &str) -> Result<Value, vm::Error> {
    let module = syntax::parse(input).unwrap();
    let module = emit::emit(&self.emit_ctx, "code", &module).unwrap();
    let main = module.borrow().main().clone();
    self.vm.call(main.into(), &[], Value::none())
  }

  fn validate(&mut self, input: &str) -> Result<ParseResult, String> {
    use ParseResult::*;

    fn is_empty(line: &str) -> bool {
      line.trim().is_empty()
    }

    fn is_indented(line: &str) -> bool {
      line
        .trim_start_matches(|c| c == '\n')
        .starts_with(|c: char| c.is_ascii_whitespace())
    }

    fn begins_block(line: &str) -> bool {
      line.trim_end_matches(|c| c == '\n').ends_with(':')
    }

    let is_multi_line = input.find('\n').is_some();
    if is_multi_line {
      let last_line = input.split('\n').last().unwrap();
      if !is_empty(last_line) && (is_indented(last_line) || begins_block(last_line)) {
        return Ok(Incomplete);
      }
    } else if begins_block(input) {
      return Ok(Incomplete);
    }

    match syntax::parse(input) {
      Ok(_) => Ok(ParseResult::Complete),
      Err(errors) => {
        let mut out = String::new();
        for error in errors {
          error.report_to(input, &mut out);
        }
        Err(out)
      }
    }
  }
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> rustyline::Result<()> {
  let mut repl = Repl::new();
  let mut buffer = String::new();

  println!("Mu {VERSION} REPL. Press CTRL-D to exit.");

  loop {
    if let Err(e) = repl.read_multi_line_input(&mut buffer) {
      match e {
        Error::Readline(e) => return Err(e),
        Error::Parse(e) => {
          println!("{e}");
          continue;
        }
      }
    };

    match repl.eval(&buffer) {
      Ok(v) => println!("{v}"),
      Err(e) => {
        println!("{}", e.report(buffer.clone()))
      }
    }
  }
}
