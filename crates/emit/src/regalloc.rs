//! Register allocator
//!
//! We use a modification of the [linear scan](https://web.archive.org/web/20221205135642/http://web.cs.ucla.edu/~palsberg/course/cs132/linearscan.pdf)
//! algorithm with infinite registers. This means we do not perform register
//! spills. Register allocation is local to each function.
//!
//! In summary, the algorithm works by tracking the liveness intervals of
//! registers, and scanning the intervals to determine when registers may be
//! reused.
//!
//! It works in two phases:
//! 1. Liveness analysis
//! 2. Allocation
//!
//! ### Liveness analysis
//!
//! During liveness analysis, registers are allocated for variables and
//! intermediate values used in expressions, and each usage of active registers
//! is tracked.
//!
//! ### Allocation
//!
//! During allocation, the live intervals are traversed for the purpose of
//! constructing an index mapping each register to its final slot.
//! This mapping is done on a first-fit basis. The final slot is the first
//! free register at the time when the register was allocated.
//!
//! ### Example
//!
//! After tracking, the live intervals are:
//!
//! ```text,ignore
//! a │ ●━━━━━●
//! b │    ●━━━━━━━━━━━━━━━━━━━━━━━●
//! c │          ●━━━━━━━━━━━━━━●
//! d │             ●━━━━━━━━●
//! e │                ●━━●
//!   ┕━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
//!     0  1  2  3  4  5  6  7  8  9  10
//! ```
//!
//! Given the live intervals above, the scan would proceed step-by-step as
//! follows:
//!
//! 0. a=r0 <- no free registers, `r0` is allocated
//! 1. a=r0, b=r1 <- no free registers, `r1` is allocated
//! 2. a=r0, b=r1 <- `r0` is freed
//! 3. b=r1, c=r0 <- `r0` is reused
//! 4. b=r1, c=r0, d=r2 <- no free registers, `r2` is allocated
//! 5. b=r1, c=r0, d=r2, e=r3 <- no free registers, `r3` is allocated
//! 6. b=r1, c=r0, d=r2, e=r3 <- `r3` is freed
//! 7. b=r1, c=r0, d=r2 <- `r2` is freed
//! 8. b=r1, c=r0, <- `r0` is freed
//! 9. b=r1 <- `r1` is freed
//! 10. done
//!
//! Each `<name>=<register>` pair is a mapping from a tracking register to a
//! final register. This information is then used to patch the bytecode.
//!
//! The maximum register index at any point was `3`, meaning this function will
//! need 4 registers.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use indexmap::IndexMap;

#[derive(Clone)]
pub struct RegAlloc(Rc<RefCell<Tracking>>);

impl RegAlloc {
  #[allow(clippy::new_without_default)]
  pub fn new() -> Self {
    Self(Rc::new(RefCell::new(Tracking {
      intervals: Vec::new(),
      event: 0,
    })))
  }

  pub fn alloc(&mut self) -> Register {
    let reg = Register {
      index: self.0.borrow().intervals.len() as u32,
      tracking: self.0.clone(),
    };
    self.0.borrow_mut().event(|this, event| {
      this.intervals.push(Interval {
        index: reg.index,
        start: event,
        end: event,
      })
    });
    reg
  }

  /// Returns a tuple of:
  /// 0. The total number of used registers
  /// 1. A mapping from each register index to a final register slot
  pub fn scan(&self) -> (u32, IndexMap<u32, u32>) {
    // println!("{}", DisplayTracking(&self.0.borrow()));
    linear_scan(&self.0.borrow().intervals)
  }

  #[cfg(test)]
  pub(crate) fn get_tracking(&self) -> Rc<RefCell<Tracking>> {
    self.0.clone()
  }
}

fn linear_scan(intervals: &[Interval]) -> (u32, IndexMap<u32, u32>) {
  let mut mapping = IndexMap::new();

  let mut free = VecDeque::new();
  let mut active = Active::new();
  let mut registers = 0u32;

  // intervals sorted in order of increasing start point
  let mut intervals_by_start = intervals.to_vec();
  intervals_by_start.sort_unstable_by_key(|i| i.start);

  // foreach live interval i, in order of increasing start point
  for i in intervals_by_start.iter() {
    // expire old intervals
    expire_old_intervals(i, &mut free, &mut active);
    // Note: we never spill
    // register[i] ← a register removed from pool of free registers
    // Note: in our case, we either remove from the pool, or allocate a new one
    let register = allocate(&mut free, &mut registers);
    // add i to active, sorted by increasing end point
    // Note: we only do this to keep track of which registers are in use,
    //       because we do not need to perform spills
    active.map.insert(i.index, (*i, register));
    // in our case, we construct a mapping from intervals to final registers
    // this is later used to patch the bytecode
    mapping.insert(i.index, register);
  }

  fn expire_old_intervals(i: &Interval, free: &mut VecDeque<u32>, active: &mut Active) {
    // foreach interval j in active, in order of increasing end point
    active.sort_by_end();
    for j in active.scratch.iter() {
      // if endpoint[j] ≥ startpoint[i] then
      if j.end > i.start {
        // return
        return;
      }

      // remove j from active
      let register = active.map.remove(&j.index).unwrap().1;
      // add register[j] to pool of free registers
      free.push_back(register);
    }
  }

  fn allocate(free: &mut VecDeque<u32>, registers: &mut u32) -> u32 {
    // attempt to acquire a free register, and fall back to allocating a new one
    if let Some(reg) = free.pop_front() {
      reg
    } else {
      let reg = *registers;
      *registers += 1;
      reg
    }
  }

  // TODO: use indexmap instead of this
  struct Active {
    map: HashMap<u32, (Interval, u32)>,
    scratch: Vec<Interval>,
  }

  impl Active {
    pub fn new() -> Self {
      Self {
        map: HashMap::new(),
        scratch: Vec::new(),
      }
    }

    pub fn sort_by_end(&mut self) {
      self.scratch.clear();
      self.scratch.extend(self.map.values().map(|v| v.0));
      self.scratch.sort_unstable_by(|a, b| a.end.cmp(&b.end));
    }
  }

  (registers, mapping)
}

pub(crate) struct Tracking {
  intervals: Vec<Interval>,
  event: usize,
}

impl Tracking {
  fn event(&mut self, f: impl FnOnce(&mut Self, usize)) {
    let event = self.event;
    f(self, event);
    self.event += 1;
  }

  fn access(&mut self, reg: u32) {
    self.event(|this, event| {
      this.intervals[reg as usize].end = event;
    })
  }
}

#[derive(Clone)]
pub struct Register {
  index: u32,
  tracking: Rc<RefCell<Tracking>>,
}

impl Register {
  /// Get the index of this register.
  ///
  /// This should be called each time the register is used.
  pub fn index(&self) -> u32 {
    self.tracking.borrow_mut().access(self.index);
    self.index
  }
}

#[derive(Clone, Copy)]
struct Interval {
  index: u32,
  start: usize,
  end: usize,
}

pub(crate) struct DisplayTracking<'a>(pub(crate) &'a Tracking);

impl<'a> std::fmt::Display for DisplayTracking<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let this = &self.0;

    fn num_digits(v: usize) -> usize {
      use std::iter::successors;

      successors(Some(v), |&n| (n >= 10).then_some(n / 10)).count()
    }

    if this.intervals.is_empty() {
      write!(f, "<empty>")?;
      return Ok(());
    }

    let index_align = num_digits(this.intervals.len()) - 1;
    let mut steps = 0;
    for (index, interval) in this.intervals.iter().enumerate() {
      write!(f, "r{index}{:w$} │ ", "", w = index_align)?;
      for _ in 0..interval.start {
        write!(f, "    ")?;
      }
      write!(f, "●━━━")?;
      for v in interval.start + 1..interval.end {
        if v == interval.end {
          break;
        }
        write!(f, "━━━━")?;
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
      steps = 2 + steps * 3
    )?;
    write!(f, "  {0:w$}   ", "", w = index_align)?;
    for i in 0..=steps {
      write!(f, "{i:02}  ")?;
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests;
