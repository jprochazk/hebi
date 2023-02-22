use std::fmt::Display;

use indexmap::IndexMap;

use super::{Access, Dict, Func, Str};
use crate::ctx::Context;
use crate::util::JoinIter;
use crate::value::handle::Handle;

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

impl Display for Registry {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<registry>")
  }
}

impl Access for Registry {}

pub struct Module {
  name: Handle<Str>,
  main: Handle<Func>,
  pub globals: Handle<Dict>,
}

// TODO: get globals from module
impl Module {
  pub fn new(ctx: Context, name: Handle<Str>, main: Handle<Func>) -> Self {
    Self {
      name,
      main,
      globals: ctx.alloc(Dict::new()),
    }
  }
}

#[derive::delegate_to_handle]
impl Module {
  pub fn name(&self) -> Handle<Str> {
    self.name.clone()
  }

  pub fn main(&self) -> Handle<Func> {
    self.main.clone()
  }
}

impl Display for Module {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<module {}>", self.name())
  }
}

impl Access for Module {}

pub struct Path {
  segments: Vec<Str>,
}

impl Path {
  pub fn new(segments: Vec<Str>) -> Self {
    Self { segments }
  }
}

#[derive::delegate_to_handle]
impl Path {
  pub fn segments(&self) -> &[Str] {
    &self.segments
  }
}

impl Display for Path {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "<path {}>",
      self.segments().iter().map(|s| s.as_str()).join(".")
    )
  }
}

impl Access for Path {}
