use std::cell::RefCell;
use std::cmp::{Ordering, Reverse};
use std::collections::HashMap;
use std::ops::{Range, RangeBounds};
use std::rc::Rc;

use super::op;

#[derive(Default)]
pub struct RegAlloc(Rc<RefCell<State>>);

#[derive(Default)]
struct State {
  intervals: Vec<Interval>,
  event: usize,
  register: usize,
}

impl State {
  fn event(&mut self) -> usize {
    let event = self.event;
    self.event += 1;
    event
  }

  fn registers(&mut self, n: usize) -> Range<usize> {
    let start = self.register;
    self.register += n;
    start..start + n
  }

  fn alloc(&mut self) -> (usize, usize) {
    let index = self.intervals.len();
    let register = self.registers(1).start;
    let event = self.event();

    self.intervals.push(Interval {
      start: event,
      end: event,
      entry: Entry::Register(register),
    });

    (index, register)
  }

  fn alloc_slice(&mut self, n: usize) -> (usize, Range<usize>) {
    let index = self.intervals.len();
    let slice = self.registers(n);
    let event = self.event();

    self.intervals.push(Interval {
      start: event,
      end: event,
      entry: Entry::Slice(slice.clone()),
    });

    (index, slice)
  }

  fn access(&mut self, index: usize) {
    let event = self.event();
    self.intervals[index].end = event;
  }
}

#[derive(Clone)]
struct Interval {
  start: usize,
  end: usize,
  entry: Entry,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum Entry {
  Register(usize),
  Slice(Range<usize>),
}

impl RegAlloc {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn alloc(&mut self) -> Register {
    let (index, register) = self.0.borrow_mut().alloc();
    Register(RegisterKind::Direct {
      state: self.0.clone(),
      index,
      register,
    })
  }

  pub fn alloc_slice(&mut self, n: usize) -> Slice {
    let (index, slice) = self.0.borrow_mut().alloc_slice(n);
    Slice {
      state: self.0.clone(),
      slice,
      index,
    }
  }

  pub fn finish(&self) -> (usize, Vec<usize>) {
    linear_scan(&self.0.borrow().intervals)
  }
}

#[derive(Clone)]
pub struct Register(RegisterKind);

#[derive(Clone)]
enum RegisterKind {
  Direct {
    state: Rc<RefCell<State>>,
    index: usize,
    register: usize,
  },
  Ref {
    item: SliceItem,
  },
}

impl Register {
  pub fn access(&self) -> op::Register {
    match &self.0 {
      RegisterKind::Direct {
        state,
        index,
        register,
      } => {
        state.borrow_mut().access(*index);
        op::Register(*register as u32)
      }
      RegisterKind::Ref { item } => item.access(),
    }
  }
}

#[derive(Clone)]
pub struct Slice {
  state: Rc<RefCell<State>>,
  slice: Range<usize>,
  index: usize,
}

impl Slice {
  pub fn access(&self, n: usize) -> op::Register {
    self.state.borrow_mut().access(self.index);
    assert!(self.slice.start + n < self.slice.end);
    op::Register((self.slice.start + n) as u32)
  }

  pub fn get(&self, n: usize) -> Register {
    Register(RegisterKind::Ref {
      item: SliceItem {
        slice: self.clone(),
        n,
      },
    })
  }

  pub fn offset(&self, offset: usize) -> SliceRef {
    SliceRef {
      slice: self.clone(),
      offset,
    }
  }

  pub fn len(&self) -> usize {
    self.slice.len()
  }
}

pub struct SliceRef {
  slice: Slice,
  offset: usize,
}

impl SliceRef {
  pub fn access(&self, n: usize) -> op::Register {
    self.slice.access(self.offset + n)
  }

  pub fn get(&self, n: usize) -> Register {
    self.slice.get(self.offset + n)
  }
}

#[derive(Clone)]
struct SliceItem {
  slice: Slice,
  n: usize,
}

impl SliceItem {
  pub fn access(&self) -> op::Register {
    self.slice.access(self.n)
  }
}

#[derive(Clone)]
enum Allocation {
  Register(usize),
  Slice(Range<usize>),
}

type Free = SortedVec<Reverse<usize>>;
type Active = HashMap<Entry, (Interval, Allocation)>;

fn linear_scan(intervals: &[Interval]) -> (usize, Vec<usize>) {
  let mut mapping = Vec::new();
  mapping.resize(intervals.len(), 0usize);

  let mut free = Free::new();
  let mut active = Active::new();
  let mut registers = 0usize;

  for interval in intervals {
    expire_old_intervals(interval, &mut free, &mut active);
    match &interval.entry {
      Entry::Register(index) => {
        let register = allocate(&mut free, &mut registers);
        active.insert(
          interval.entry.clone(),
          (interval.clone(), Allocation::Register(register)),
        );
        mapping.insert(*index, register);
      }
      Entry::Slice(indices) => {
        let slice = allocate_slice(indices.len(), &mut free, &mut registers);
        active.insert(
          interval.entry.clone(),
          (interval.clone(), Allocation::Slice(slice.clone())),
        );
        for (index, register) in indices.clone().zip(slice) {
          mapping.insert(index, register);
        }
      }
    }
  }

  (registers, mapping)
}

fn allocate(free: &mut Free, registers: &mut usize) -> usize {
  // attempt to acquire a free register, and fall back to allocating a new one
  if let Some(Reverse(reg)) = free.pop() {
    reg
  } else {
    let reg = *registers;
    *registers += 1;
    reg
  }
}

fn allocate_slice(n: usize, free: &mut Free, registers: &mut usize) -> Range<usize> {
  assert!(n > 0);

  if n == 1 {
    let reg = allocate(free, registers);
    reg..reg + 1
  } else {
    // fast path: no free registers OR no registers allocated yet
    if free.is_empty() || *registers == 0 {
      let start = *registers;
      *registers += n;
      return start..*registers;
    }

    // TODO: combine `find_contiguous_registers` loop and the partial alloc loop
    // into one
    //
    // if `free[range.start] == *registers - 1`
    // and `range.len() != N` then return it as a partial range
    // which will be filled in with fresh registers.
    //
    // then benchmark it against current setup, because it's not clear which will be
    // faster. current setup could be faster, because we are way more likely to
    // find a contiguous range at the top of the register file. new setup could
    // be faster, because it would have better theoretical time complexity,
    // because we would only iterate once, but we'd have to iterate from the bottom
    // of the register file instead of from the top, which means we may be far
    // less likely to actually hit a valid contiguous range.

    // slow path: find contiguous register range
    if let Some(range) = find_contiguous_registers(n, free) {
      let slice = &free[range.clone()];
      let end = slice.first().unwrap().0 + 1;
      let start = slice.last().unwrap().0;
      let _ = free.drain(range);
      return start..end;
    }

    // slower path: find at least 1 and at most N-1 top-most free registers
    // and fill the rest of N with fresh registers
    if free[0].0 == *registers - 1 {
      let mut count = 1;
      for i in 1..free.len() {
        let (a, b) = (free[i - 1].0, free[i].0);
        if a != b + 1 {
          break;
        }
        count += 1;
      }

      let slice = &free[0..count];
      let mut end = slice.first().unwrap().0 + 1;
      let start = slice.last().unwrap().0;
      let _ = free.drain(0..count);

      while end < n {
        end += 1;
        *registers += 1;
      }

      return start..end;
    }

    // slowest path: allocate entire slice using fresh registers
    let start = *registers;
    *registers += n;
    start..*registers
  }
}

/// Returns a range which can be used to slice `free`
/// to get a slice of contiguous registers.
fn find_contiguous_registers(n: usize, free: &Free) -> Option<Range<usize>> {
  assert!(n >= 2);

  if free.len() < n {
    return None;
  }

  let mut start = 0;
  let mut count = 1;
  for i in 0..free.len() - 1 {
    let (a, b) = (free[i].0, free[i + 1].0);
    if a == b + 1 {
      count += 1;
      if count == n {
        return Some(start..start + count);
      }
    } else {
      start += count;
      count = 1;
    }
  }

  None
}

fn expire_old_intervals(i: &Interval, free: &mut Free, active: &mut Active) {
  active.retain(|_, (j, allocation)| {
    if j.end < i.start {
      match allocation {
        Allocation::Register(register) => free.insert(Reverse(*register)),
        Allocation::Slice(slice) => {
          // TODO: bulk insert
          for register in slice.clone() {
            free.insert(Reverse(register));
          }
        }
      }
      false
    } else {
      true
    }
  });
}

#[derive(Default)]
struct SortedVec<T> {
  inner: Vec<T>,
}

impl<T: Ord> SortedVec<T> {
  fn new() -> Self {
    SortedVec { inner: vec![] }
  }

  fn len(&self) -> usize {
    self.inner.len()
  }

  fn is_empty(&self) -> bool {
    self.inner.is_empty()
  }

  /// Insert the element into the sorted vec.
  ///
  /// This attemps to insert the element to the top of the container,
  /// and falls back to binary search + insert if it fails.
  ///
  /// The time complexity is best case O(1), and worst case O(N+log(N)).
  fn insert(&mut self, element: T) {
    // If `inner` is empty or `element >= last`, push `element` onto the end
    if let None | Some(Ordering::Equal | Ordering::Greater) =
      self.inner.last().map(|v| element.cmp(v))
    {
      self.inner.push(element);
      return;
    }

    // `inner` is not empty and `element < last`, find insertion point and insert
    let index = match self.inner.binary_search(&element) {
      Ok(index) | Err(index) => index,
    };

    self.inner.insert(index, element);
  }

  fn pop(&mut self) -> Option<T> {
    self.inner.pop()
  }

  fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> std::vec::Drain<'_, T> {
    self.inner.drain(range)
  }

  fn as_slice(&self) -> &[T] {
    self.inner.as_slice()
  }
}

/* fn init_array_with<T: Sized, const N: usize>(mut f: impl FnMut(usize) -> T) -> [T; N] {
  let mut array: [_; N] = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
  for (i, value) in array.iter_mut().enumerate() {
    *value = std::mem::MaybeUninit::new(f(i));
  }
  let out = unsafe { std::ptr::read(&mut array as *mut _ as *mut [T; N]) };
  std::mem::forget(array);
  out
} */

impl<T, I: std::slice::SliceIndex<[T]>> std::ops::Index<I> for SortedVec<T> {
  type Output = I::Output;

  #[inline]
  fn index(&self, index: I) -> &Self::Output {
    std::ops::Index::index(&self.inner, index)
  }
}

#[cfg(test)]
mod tests;
