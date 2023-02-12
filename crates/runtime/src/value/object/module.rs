use indexmap::IndexMap;

use super::handle::Handle;
use super::{Dict, Func};

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

#[derive(Clone, Debug)]
pub struct Module {
  name: String,
  main: Handle<Func>,
  pub globals: Handle<Dict>,
}

impl Module {
  pub fn new(name: impl Into<String>, main: Handle<Func>) -> Self {
    Self {
      name: name.into(),
      main,
      globals: Dict::new().into(),
    }
  }

  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn main(&self) -> &Handle<Func> {
    &self.main
  }
}

#[derive(Clone, Debug)]
pub struct Path {
  segments: Vec<String>,
}

impl Path {
  pub fn new(segments: Vec<String>) -> Self {
    Self { segments }
  }

  pub fn segments(&self) -> &[String] {
    &self.segments
  }
}
