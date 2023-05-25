use std::error::Error;
use std::fmt::Display;
use std::io;
use std::io::Write;
use std::time::Duration;

use crossterm::cursor::MoveTo;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};
use crossterm::{cursor, terminal, QueueableCommand};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync + 'static>>;

/* macro_rules! fail {
  ($($tt:tt)*) => {
    return Err(<Box<dyn Error + Send + Sync + 'static>>::from(format!($($tt)*)));
  }
} */

struct Line<'a>(&'a [char]);

impl<'a> Display for Line<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    for char in self.0.iter() {
      write!(f, "{char}")?;
    }
    Ok(())
  }
}

fn count_lines_with_wrapping(buffer: &[char], size: (u16, u16)) -> usize {
  let mut lines = 0;
  // for line in buffer {
  lines += buffer.len() / size.0 as usize;
  // }
  lines
}

fn main() -> Result<()> {
  terminal::enable_raw_mode()?;

  let mut stdout = io::stdout();
  {
    let w = &mut stdout;
    let mut buffer = Vec::<char>::new();
    let anchor = cursor::position()?;

    loop {
      event::poll(Duration::from_millis(10))?;

      let mut position = cursor::position()?;
      let start_lines = count_lines_with_wrapping(&buffer, terminal::size()?);

      match event::read()? {
        Event::Key(KeyEvent {
          code: KeyCode::Char('c'),
          modifiers: KeyModifiers::CONTROL,
          ..
        })
        | Event::Key(KeyEvent {
          code: KeyCode::Char('d'),
          modifiers: KeyModifiers::CONTROL,
          ..
        }) => {
          break;
        }
        Event::Key(KeyEvent {
          code: KeyCode::Char(c),
          ..
        }) => {
          buffer.insert(position.0 as usize, c);
          position.0 += 1;
        }
        Event::Key(KeyEvent {
          code: KeyCode::Backspace,
          ..
        }) => {
          if position.0 > 0 {
            buffer.remove(position.0 as usize - 1);
            position.0 -= 1;
          }
        }
        // Event::Key(_) => todo!(),
        // Event::Paste(_) => todo!(),
        // Event::Resize(_, _) => todo!(),
        _ => {}
      }

      let end_lines = count_lines_with_wrapping(&buffer, terminal::size()?);

      /* if start_lines != end_lines {
        if start_lines > end_lines {
          anchor -=
        }
      } */

      w.queue(MoveTo(anchor.0, anchor.1))?;
      w.queue(Clear(ClearType::FromCursorDown))?;
      w.queue(Print(Line(&buffer)))?;
      w.queue(MoveTo(position.0, position.1))?;

      write!(w, "{}", start_lines.abs_diff(end_lines))?;

      w.flush()?;
    }
  }

  terminal::disable_raw_mode()?;

  Ok(())
}
