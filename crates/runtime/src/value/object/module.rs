use indexmap::IndexMap;

use super::handle::Handle;
use super::{Access, Dict, Func, Str};

#[derive(Clone, Debug)]
pub struct Registry {
  pub modules: IndexMap<String, Module>,
}

impl Registry {
  pub fn new() -> Self {
    Self {
      modules: IndexMap::new(),
    }
  }
}

impl Default for Registry {
  fn default() -> Self {
    Self::new()
  }
}

impl Access for Registry {}

#[derive(Clone, Debug)]
pub struct Module {
  name: Str,
  main: Handle<Func>,
  pub globals: Handle<Dict>,
}

// TODO: get globals from module
impl Module {
  pub fn new(name: impl Into<Str>, main: Handle<Func>) -> Self {
    Self {
      name: name.into(),
      main,
      globals: Dict::new().into(),
    }
  }

  pub fn name(&self) -> &str {
    self.name.as_str()
  }

  pub fn main(&self) -> &Handle<Func> {
    &self.main
  }
}

impl Access for Module {}

#[derive(Clone, Debug)]
pub struct Path {
  segments: Vec<Str>,
}

impl Path {
  pub fn new(segments: Vec<Str>) -> Self {
    Self { segments }
  }

  pub fn segments(&self) -> &[Str] {
    &self.segments
  }
}

impl Access for Path {}
