pub fn leading_whitespace(str: &str) -> &str {
  let non_ws = str.find(|c: char| !c.is_ascii_whitespace()).unwrap_or(0);
  &str[..non_ws]
}
