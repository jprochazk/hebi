use std::fmt::Display;

use indexmap::IndexSet;

use crate::public::{Context, Dict};
use crate::{Error, IntoHebi, Result, Value};

pub struct Join<Iter, Sep>(pub Iter, pub Sep);

impl<Iter, Sep> Display for Join<Iter, Sep>
where
  Iter: Iterator + Clone,
  <Iter as Iterator>::Item: Display,
  Sep: Display,
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let sep = &self.1;
    let mut peekable = self.0.clone().peekable();
    while let Some(item) = peekable.next() {
      write!(f, "{item}")?;
      if peekable.peek().is_some() {
        write!(f, "{sep}")?;
      }
    }
    Ok(())
  }
}

pub trait JoinIter: Sized {
  fn join<Sep>(&self, sep: Sep) -> Join<Self, Sep>;
}

impl<Iter> JoinIter for Iter
where
  Iter: Sized + Iterator + Clone,
{
  fn join<Sep>(&self, sep: Sep) -> Join<Self, Sep> {
    Join(self.clone(), sep)
  }
}

mod private {
  pub trait Sealed {}
}

pub fn check_args(
  args: &[Value<'_>],
  kwargs: Option<&Dict<'_>>,
  required_positional_params: &[&str],
  max_positional_params: usize,
  keyword_params: &[(&str, bool)],
) -> Result<()> {
  if args.len() < required_positional_params.len() {
    return Err(crate::Error::runtime(format!(
      "missing required positional params: {}",
      required_positional_params[args.len()..].iter().join(", "),
    )));
  }

  if args.len() > max_positional_params {
    return Err(crate::Error::runtime(format!(
      "expected at most {max_positional_params} args, got {}",
      args.len(),
    )));
  }

  let mut unknown = IndexSet::new();
  let mut missing = IndexSet::new();
  if let Some(kwargs) = kwargs {
    // we have kwargs,
    // - check for unknown keywords
    for (key, _) in kwargs.iter() {
      if !keyword_params.iter().any(|(k, _)| key == *k) {
        unknown.insert(key.to_string());
      }
    }
    // - check for missing keywords
    for key in keyword_params
      .iter()
      // only check required keyword params
      .filter_map(|(k, v)| if !*v { Some(k) } else { None })
    {
      if !kwargs.has(key) {
        missing.insert(key.to_string());
      }
    }
  } else {
    // we don't have kwargs,
    // just check for missing keyword params
    missing.extend(keyword_params.iter().filter_map(|(k, v)| {
      // only check required keyword params
      if !*v {
        Some(k.to_string())
      } else {
        None
      }
    }))
  }
  // if we have a mismatch, output a comprehensive error
  if !unknown.is_empty() || !missing.is_empty() {
    return Err(Error::runtime(format!(
      "mismatched keyword params: {}{}{}",
      if !unknown.is_empty() {
        format!("could not recognize {}", unknown.iter().join(", "))
      } else {
        String::new()
      },
      if !unknown.is_empty() && !missing.is_empty() {
        " and "
      } else {
        ""
      },
      if !missing.is_empty() {
        format!("missing {}", missing.iter().join(", "))
      } else {
        String::new()
      },
    )));
  }
  Ok(())
}
