use super::*;

macro_rules! check {
  ($v:expr) => {
    let (registers, mapping) = $v;
    let mut mapping = mapping.into_iter().collect::<Vec<_>>();
    mapping.sort_unstable_by_key(|&v| v.0);
    if cfg!(feature = "emit_snapshots") {
      insta::assert_snapshot!(format!("registers={registers}\n{mapping:#?}"));
    }
  };
}

#[test]
fn reg_alloc_1() {
  //
  // a │ ●━━━━━●
  // b │    ●━━━━━━━━━━━━━━━━━●
  // c │                ●━━●
  //   ┕━━━━━━━━━━━━━━━━━━━━━━━━━━━
  //     0  1  2  3  4  5  6  7  8
  //
  // becomes
  //
  // r0 │ a━━━━━a        c━━c
  // r1 │    b━━━━━━━━━━━━━━━━━b
  //    ┕━━━━━━━━━━━━━━━━━━━━━━━━━━━
  //      0  1  2  3  4  5  6  7  8
  //

  let mut regalloc = RegAlloc::new();

  if cfg!(feature = "emit_snapshots") {
    insta::assert_snapshot!(format!("{}", DisplayTracking(&regalloc.0.borrow())));
  }

  let r0 = regalloc.alloc();
  let r1 = regalloc.alloc();

  r0.index();
  r1.index();
  r1.index();

  let t0 = regalloc.alloc();
  t0.index();

  r1.index();

  if cfg!(feature = "emit_snapshots") {
    insta::assert_snapshot!(format!("{}", DisplayTracking(&regalloc.0.borrow())));
  }

  check!(regalloc.scan());
}

#[test]
fn reg_alloc_2() {
  //
  // a │ ●━━━━━●
  // b │    ●━━━━━━━━━━━━━━━━━━━━━━━●
  // c │          ●━━━━━━━━━━━━━━●
  // d │             ●━━━━━━━━●
  // e │                ●━━●
  //   ┕━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  //     0  1  2  3  4  5  6  7  8  9  10
  //
  // becomes
  //
  // r0 │ a━━━━━a  c━━━━━━━━━━━━━━c
  // r1 │    b━━━━━━━━━━━━━━━━━━━━━━━b
  // r2 │             d━━━━━━━━d
  // r3 │                e━━e
  //    ┕━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  //      0  1  2  3  4  5  6  7  8  9  10
  //

  let mut regalloc = RegAlloc::new();
  let a = regalloc.alloc();
  let b = regalloc.alloc();
  a.index();
  let c = regalloc.alloc();
  let d = regalloc.alloc();
  let e = regalloc.alloc();
  e.index();
  d.index();
  c.index();
  b.index();

  if cfg!(feature = "emit_snapshots") {
    insta::assert_snapshot!(format!("{}", DisplayTracking(&regalloc.0.borrow())));
  }
  check!(regalloc.scan());
}
