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

/* #[test]
fn slice() {
  let mut regalloc = RegAlloc::new();

  let a = regalloc.alloc();
  let b = regalloc.alloc();
  let c = regalloc.alloc();
  let d = regalloc.alloc();
} */

#[test]
fn try_find_contiguous() {
  {
    // index: 0 1 2 3 4         5
    // reg:   9 8 7 6 5 4 3 2 1 0
    // free:  + + + + + x x x x +

    let mut free = Free::new();
    for i in (0..10).rev() {
      free.insert(Reverse(i));
    }
    let _ = free.drain(5..9);

    assert_eq!(Some(0..2), find_contiguous_registers(2, &free));
    assert_eq!(Some(0..4), find_contiguous_registers(4, &free));
    assert_eq!(Some(0..5), find_contiguous_registers(5, &free));
    assert_eq!(None, find_contiguous_registers(6, &free));
  }
  {
    // index: 0   1 2   3 4 5   6 7 8 9
    // reg:   C B A 9 8 7 6 5 4 3 2 1 0
    // free:  + x + + x + + + x + + + +

    let mut free = Free::new();
    for i in (0..13).rev() {
      free.insert(Reverse(i));
    }
    let _ = free.drain(8..9);
    let _ = free.drain(4..5);
    let _ = free.drain(1..2);

    assert_eq!(Some(1..3), find_contiguous_registers(2, &free));
    assert_eq!(Some(3..6), find_contiguous_registers(3, &free));
    assert_eq!(Some(6..10), find_contiguous_registers(4, &free));
    assert_eq!(None, find_contiguous_registers(5, &free));
  }
}

#[test]
fn alloc_register_slice() {
  let mut regalloc = RegAlloc::new();

  let a = regalloc.alloc();
  let slice = regalloc.alloc_slice(4);
  for _ in 0..2 {
    assert_eq!(slice.access(0).0, 1);
    assert_eq!(slice.access(1).0, 2);
    assert_eq!(slice.access(2).0, 3);
    assert_eq!(slice.access(3).0, 4);
    assert_eq!(a.access().0, 0);
  }

  let (registers, map) = regalloc.finish();

  assert_snapshot!(DisplayGraph(&regalloc.0.borrow(), registers, &map).to_string());
}

#[test]
fn sorted_vec_insert() {
  let mut vec = SortedVec::new();
  for i in 0..10 {
    vec.insert(i);
  }
  let _ = vec.drain(2..8);
  assert_eq!(vec.inner, [0, 1, 8, 9]);
  vec.insert(5);
  assert_eq!(vec.inner, [0, 1, 5, 8, 9]);
  vec.insert(4);
  assert_eq!(vec.inner, [0, 1, 4, 5, 8, 9]);
  vec.insert(6);
  assert_eq!(vec.inner, [0, 1, 4, 5, 6, 8, 9]);
  vec.insert(3);
  assert_eq!(vec.inner, [0, 1, 3, 4, 5, 6, 8, 9]);
  vec.insert(7);
  assert_eq!(vec.inner, [0, 1, 3, 4, 5, 6, 7, 8, 9]);
  vec.insert(2);
  assert_eq!(vec.inner, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
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

    struct BasicInterval {
      start: usize,
      end: usize,
      index: usize,
    }

    let mut intervals = vec![];
    for interval in this.intervals.iter() {
      match &interval.entry {
        Entry::Register(index) => intervals.push(BasicInterval {
          start: interval.start,
          end: interval.end,
          index: *index,
        }),
        Entry::Slice(slice) => {
          for index in slice.clone() {
            intervals.push(BasicInterval {
              start: interval.start,
              end: interval.end,
              index,
            });
          }
        }
      }
    }

    let index_align = num_digits(intervals.len()) - 1;
    let step_align = num_digits(this.event);

    let mut steps = 0;
    for (index, interval) in intervals.iter().enumerate() {
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
