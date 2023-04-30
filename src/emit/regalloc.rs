use std::cell::RefCell;
use std::cmp::{Ordering, Reverse};
use std::collections::HashMap;
use std::ops::{Range, RangeBounds};
use std::rc::Rc;

use super::op;

// TODO: look into how V8 does register allocation
// this clearly isn't sustainable, because there is no way to
// allocate a range of *contiguous* registers. this is a problem,
// because a lot of instructions actually depend on registers
// being contiguous (call, print, make_*). V8 also uses contiguous
// register ranges (such as in calls), and they somehow make it work.
// they also do register allocation on the fly, which would also be
// beneficial here.

#[derive(Default)]
pub struct RegAlloc(Rc<RefCell<State>>);

#[derive(Default)]
struct State {
  preserve: Vec<Option<Register>>,
  intervals: Vec<Interval>,
  event: usize,
}

impl State {
  fn event(&mut self) -> usize {
    let event = self.event;
    self.event += 1;
    event
  }

  fn alloc(&mut self) -> usize {
    let index = self.intervals.len();
    let event = self.event();
    self.intervals.push(Interval {
      index,
      start: event,
      end: event,
    });
    index
  }

  fn access(&mut self, index: usize) {
    let event = self.event();
    self.intervals[index].end = event;
  }
}

#[derive(Clone, Copy)]
struct Interval {
  index: usize,
  start: usize,
  end: usize,
}

impl RegAlloc {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn alloc(&mut self) -> Register {
    let index = self.0.borrow_mut().alloc();
    Register {
      state: self.0.clone(),
      index,
    }
  }

  pub fn alloc_many<const N: usize>(&mut self) -> [Register; N] {
    init_array_with(|_| Register {
      state: self.0.clone(),
      index: self.0.borrow_mut().alloc(),
    })
  }

  pub fn finish(&self) -> (usize, Vec<usize>) {
    linear_scan(&self.0.borrow().intervals)
  }
}

#[derive(Clone)]
pub struct Register {
  state: Rc<RefCell<State>>,
  index: usize,
}

impl Register {
  pub fn access(&self) -> op::Register {
    self.state.borrow_mut().access(self.index);
    op::Register(self.index as u32)
  }
}

type Free = SortedVec<Reverse<usize>>;

// TODO: discard this work.

fn linear_scan(intervals: &[Interval]) -> (usize, Vec<usize>) {
  let mut mapping = Vec::new();
  mapping.resize(intervals.len(), 0usize);

  let mut free = Free::new();
  let mut active = Active::new();
  let mut registers = 0usize;

  for interval in intervals {
    expire_old_intervals(interval, &mut free, &mut active);
    let register = allocate(&mut free, &mut registers);
    active.map.insert(interval.index, (*interval, register));
    mapping.insert(interval.index, register);
  }

  (registers, mapping)
}

struct Active {
  map: HashMap<usize, (Interval, usize)>,
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
    match find_contiguous_registers(n, free) {
      Some(slice) => slice,
      None => {
        let start = *registers;
        *registers += n;
        start..*registers
      }
    }
  }
}

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
  active.sort_by_end();
  for j in active.scratch.iter() {
    if j.end >= i.start {
      return;
    }

    let (_, register) = active.map.remove(&j.index).unwrap();
    free.insert(Reverse(register));
  }
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

  /// Insert the element into the sorted vec.
  ///
  /// This attemps to insert the element to the top of the container,
  /// and falls back to binary search + insert if it fails.
  ///
  /// The complexity is best case O(1), and worst case O(N+log(N)).
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

fn init_array_with<T: Sized, const N: usize>(mut f: impl FnMut(usize) -> T) -> [T; N] {
  let mut array: [_; N] = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
  for (i, value) in array.iter_mut().enumerate() {
    *value = std::mem::MaybeUninit::new(f(i));
  }
  let out = unsafe { std::ptr::read(&mut array as *mut _ as *mut [T; N]) };
  std::mem::forget(array);
  out
}

impl<T> std::ops::Index<usize> for SortedVec<T> {
  type Output = T;

  fn index(&self, index: usize) -> &Self::Output {
    self.inner.index(index)
  }
}

#[cfg(test)]
mod tests;
