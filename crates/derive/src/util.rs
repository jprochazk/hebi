use syn::Attribute;

pub fn is_attr(attr: &Attribute, tags: &[&str]) -> bool {
  if attr.path.leading_colon.is_none() && attr.path.segments.len() == 1 {
    let ident = &attr.path.segments.first().unwrap().ident;
    tags.iter().any(|&t| ident == t)
  } else {
    false
  }
}
