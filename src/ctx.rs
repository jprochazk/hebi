#[derive(Clone)]
pub struct Context {}

impl Context {
  #[cfg(test)]
  pub(crate) fn for_test() -> Context {
    Context {}
  }
}
