use hebi::*;
use rustyline::Editor;

struct Repl {
  vm: Hebi,
  editor: Editor<()>,
  state: State,
}

#[derive(Default)]
struct State {
  print_bytecode: bool,
}

enum ParseResult {
  Incomplete,
  Complete,
}

enum Error {
  Readline(rustyline::error::ReadlineError),
  Parse(String),
}

enum Control {
  Eval,
  Loop,
}

impl Repl {
  fn new() -> Self {
    Self {
      vm: Hebi::new(),
      editor: Editor::new().unwrap(),
      state: State::default(),
    }
  }

  fn read_multi_line_input(&mut self, buffer: &mut String) -> Result<Control, Error> {
    // TODO: allow erase all input
    let mut prev_line = String::new();
    loop {
      buffer.push('\n');
      let ws = &prev_line[..prev_line
        .chars()
        .take_while(|c| c.is_ascii_whitespace())
        .count()];
      let line = self
        .editor
        .readline_with_initial("> ", (ws, ""))
        .map_err(Error::Readline)?;
      prev_line.clear();
      prev_line.push_str(&line);
      self.editor.add_history_entry(&line);
      buffer.push_str(&line);

      if self.try_cmd(buffer) {
        return Ok(Control::Loop);
      }

      match self.validate(buffer.as_str()).map_err(Error::Parse)? {
        ParseResult::Incomplete => continue,
        ParseResult::Complete => break Ok(Control::Eval),
      }
    }
  }

  fn try_cmd(&mut self, input: &str) -> bool {
    match input.trim() {
      ".print_bytecode" => {
        self.state.print_bytecode = true;
        true
      }
      _ => false,
    }
  }

  fn eval(&mut self, input: &str) -> Result<Value, EvalError> {
    self.vm.eval(input)
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

    match self.vm.check(input) {
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

pub fn run() -> rustyline::Result<()> {
  let mut repl = Repl::new();
  let mut buffer = String::new();

  println!("Hebi REPL v{VERSION}\nPress CTRL-D to exit");

  loop {
    buffer.clear();

    match repl.read_multi_line_input(&mut buffer) {
      Ok(Control::Eval) => {}
      Ok(Control::Loop) => continue,
      Err(Error::Readline(e)) => match e {
        rustyline::error::ReadlineError::Eof => return Ok(()),
        rustyline::error::ReadlineError::Interrupted => return Ok(()),
        rustyline::error::ReadlineError::WindowResized => continue,
        e => return Err(e),
      },
      Err(Error::Parse(e)) => {
        println!("{e}");
        continue;
      }
    };

    match repl.eval(&buffer) {
      Ok(v) => println!("{v}"),
      Err(EvalError::Runtime(e)) => {
        println!("{}", e.stack_trace(Some(buffer.as_str().into())))
      }
      Err(EvalError::Parse(..)) => unreachable!(),
    }
  }
}
