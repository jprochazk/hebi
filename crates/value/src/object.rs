use beef::lean::Cow;

use crate::Value;

pub type String = std::string::String;
pub type List = std::vec::Vec<Value>;
pub type Dict = std::collections::HashMap<Value, Value>;

/// Object representation
#[derive(Clone)]
enum Repr {
  String(String),
  List(List),
  Dict(Dict),
}

#[derive(Clone)]
pub struct Object {
  repr: Repr,
}

impl Object {
  pub fn string(v: impl Into<String>) -> Self {
    let v = v.into();
    v.into()
  }

  pub fn list(v: impl Into<List>) -> Self {
    let v = v.into();
    v.into()
  }

  pub fn dict(v: impl Into<Dict>) -> Self {
    let v = v.into();
    v.into()
  }
}

impl From<Dict> for Object {
  fn from(v: Dict) -> Self {
    Object {
      repr: Repr::Dict(v),
    }
  }
}

impl From<List> for Object {
  fn from(v: List) -> Self {
    Object {
      repr: Repr::List(v),
    }
  }
}

impl From<String> for Object {
  fn from(v: String) -> Self {
    Object {
      repr: Repr::String(v),
    }
  }
}

impl<'a> From<&'a str> for Object {
  fn from(value: &'a str) -> Self {
    Object {
      repr: Repr::String(value.to_string()),
    }
  }
}

impl<'a> From<Cow<'a, str>> for Object {
  fn from(value: Cow<'a, str>) -> Self {
    Object {
      repr: Repr::String(value.to_string()),
    }
  }
}

impl Object {
  pub fn as_string(&self) -> Option<&String> {
    if let Repr::String(ref v) = self.repr {
      Some(v)
    } else {
      None
    }
  }

  pub fn is_string(&self) -> bool {
    matches!(self.repr, Repr::String(..))
  }

  pub fn as_list(&self) -> Option<&List> {
    if let Repr::List(ref v) = self.repr {
      Some(v)
    } else {
      None
    }
  }

  pub fn as_list_mut(&mut self) -> Option<&mut List> {
    if let Repr::List(ref mut v) = self.repr {
      Some(v)
    } else {
      None
    }
  }

  pub fn is_list(&self) -> bool {
    matches!(self.repr, Repr::List(..))
  }

  pub fn as_dict(&self) -> Option<&Dict> {
    if let Repr::Dict(ref v) = self.repr {
      Some(v)
    } else {
      None
    }
  }

  pub fn as_dict_mut(&mut self) -> Option<&mut Dict> {
    if let Repr::Dict(ref mut v) = self.repr {
      Some(v)
    } else {
      None
    }
  }

  pub fn is_dict(&self) -> bool {
    matches!(self.repr, Repr::Dict(..))
  }
}

impl std::fmt::Debug for Object {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match &self.repr {
      Repr::String(v) => write!(f, "{v:?}"),
      Repr::List(v) => f.debug_list().entries(v).finish(),
      Repr::Dict(v) => f.debug_map().entries(v).finish(),
    }
  }
}

impl std::fmt::Display for Object {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    struct DebugAsDisplay<T: std::fmt::Display>(T);
    impl<T: std::fmt::Display> std::fmt::Debug for DebugAsDisplay<T> {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
      }
    }
    fn unit<T>(v: T) -> DebugAsDisplay<T>
    where
      T: std::fmt::Display,
    {
      DebugAsDisplay(v)
    }
    fn tuple2<A, B>((a, b): (A, B)) -> (DebugAsDisplay<A>, DebugAsDisplay<B>)
    where
      A: std::fmt::Display,
      B: std::fmt::Display,
    {
      (DebugAsDisplay(a), DebugAsDisplay(b))
    }

    match &self.repr {
      Repr::String(v) => write!(f, "\"{v}\""),
      Repr::List(v) => f.debug_list().entries(v.iter().map(unit)).finish(),
      Repr::Dict(v) => f.debug_map().entries(v.iter().map(tuple2)).finish(),
    }
  }
}
