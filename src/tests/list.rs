check! {
  list_operations,
  r#"
    v := [0,1,2] # create
    print v[0] # index
    v[0] = 3 # set index
    print v[0]
  "#
}
