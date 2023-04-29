use std::cmp;

use super::*;
use crate::util::num_digits;

#[test]
fn simple() {
  let mut regalloc = RegAlloc::new();

  let a = regalloc.alloc();
  let b = regalloc.alloc();
  a.access();
  b.access();
  b.access();
  let c = regalloc.alloc();
  c.access();
  b.access();

  let (registers, map) = regalloc.finish();

  assert_snapshot!(DisplayGraph(&regalloc.0.borrow(), registers, &map).to_string());
}

#[test]
fn overlapping() {
  let mut regalloc = RegAlloc::new();

  let a = regalloc.alloc();
  let b = regalloc.alloc();
  a.access();
  let c = regalloc.alloc();
  let d = regalloc.alloc();
  let e = regalloc.alloc();
  e.access();
  d.access();
  c.access();
  b.access();

  let (registers, map) = regalloc.finish();

  assert_snapshot!(DisplayGraph(&regalloc.0.borrow(), registers, &map).to_string());
}

struct DisplayGraph<'a>(&'a State, usize, &'a [usize]);

impl<'a> std::fmt::Display for DisplayGraph<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let DisplayGraph(this, registers, map) = self;

    if this.intervals.is_empty() {
      write!(f, "<empty>")?;
      return Ok(());
    }

    writeln!(f, "registers = {registers}")?;

    let index_align = num_digits(this.intervals.len()) - 1;
    let step_align = num_digits(this.event);

    let mut steps = 0;
    for (index, interval) in this.intervals.iter().enumerate() {
      write!(f, "r{index}{:w$} │ ", "", w = index_align)?;
      for _ in 0..interval.start {
        write!(f, "{: <w$}", "", w = (2 + step_align))?;
      }
      write!(
        f,
        "{:━<digits$}",
        map[interval.index],
        digits = cmp::max(num_digits(interval.index), 2 + step_align)
      )?;
      for v in interval.start + 1..interval.end {
        if v == interval.end {
          break;
        }
        write!(f, "{:━<w$}", "", w = (2 + step_align))?;
      }
      writeln!(f, "●")?;

      if interval.end > steps {
        steps = interval.end + 1;
      }
    }
    writeln!(
      f,
      "  {0:w$} ┕━{0:━>steps$}",
      "",
      w = index_align,
      steps = (1 + step_align) + steps * (2 + step_align)
    )?;
    write!(f, "  {0:w$}   ", "", w = index_align)?;
    for i in 0..=steps {
      write!(f, "{i: <width$}  ", width = step_align)?;
    }

    Ok(())
  }
}
