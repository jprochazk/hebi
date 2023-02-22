check! {
  print_0_to_9,
  r#"
    i := 0
    loop:
      if i >= 10:
        break
      print i
      i += 1
  "#
}
check! {
  while_print_0_to_9,
  r#"
    i := 0
    while i < 10:
      print i
      i += 1
  "#
}
check! {
  for_print_0_to_9,
  r#"
    for i in 0..10:
      print i
  "#
}
check! {
  for_print_0_to_10,
  r#"
    for i in 0..=10:
      print i
  "#
}
check! {
  for_print_nothing,
  r#"
    for i in 10..0:
      print i
  "#
}
check! {
  for_range_vars,
  r#"
    start := 0
    end := 10
    for i in start..end:
      print i
  "#
}
check! {
  for_range_vars_inclusive,
  r#"
    start := 0
    end := 10
    for i in start..=end:
      print i
  "#
}
check! {
  for_print_odd_0_to_9,
  r#"
    for i in 0..10:
      if i % 2 == 0: continue
      print i
  "#
}
