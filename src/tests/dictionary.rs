check! {
  dictionary_operations,
  r#"
    v := {a:0, b:1, c:2} # create
    print v["a"] # index
    v["a"] = 3 # set index
    print v["a"]
  "#
}
