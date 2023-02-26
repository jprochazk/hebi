check! {
  simple_module_load,
  modules: {
    "test": "value := 100"
  },
  r#"
    import test

    print test.value
  "#
}
