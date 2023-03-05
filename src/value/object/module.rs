use std::error::Error as StdError;
use std::fmt::Display;
use std::ptr::NonNull;
use std::str::FromStr;

use indexmap::{IndexMap, IndexSet};

use super::{Access, Dict, Function, FunctionDescriptor, Str};
use crate::ctx::Context;
use crate::util::JoinIter;
use crate::value::handle::Handle;
use crate::value::Value;
use crate::{Error, Result};

// TODO: all descriptor should store `String` for names, because `Handle<Str>`
// may be mutated maybe this can be solved another way, like making Str a
// primitive?

pub trait ModuleLoader {
  // TODO: how will loading native modules work?
  // maybe directly `.register` with the `Hebi` instance?
  /// Load a module at the `path`, returning its source code.
  fn load(&mut self, path: &[String]) -> Result<&str, Box<dyn StdError + 'static>>;
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModuleId(usize);

pub struct ModuleRegistry {
  next_module_id: usize,
  index: IndexMap<Vec<String>, ModuleId>,
  modules: IndexMap<ModuleId, Handle<Module>>,
}

impl ModuleRegistry {
  pub fn new() -> Self {
    Self {
      next_module_id: 1,
      index: IndexMap::new(),
      modules: IndexMap::new(),
    }
  }

  pub fn next_module_id(&mut self) -> ModuleId {
    let temp = ModuleId(self.next_module_id);
    self.next_module_id += 1;
    temp
  }

  pub fn add(&mut self, id: ModuleId, path: &[String], module: Handle<Module>) {
    self.index.insert(path.to_vec(), id);
    self.modules.insert(id, module);
  }

  pub fn remove(&mut self, id: ModuleId) -> Option<Handle<Module>> {
    self.modules.remove(&id)
  }

  pub fn by_id(&self, id: ModuleId) -> Option<Handle<Module>> {
    self.modules.get(&id).cloned()
  }

  pub fn by_path(&self, path: &[String]) -> Option<(ModuleId, Handle<Module>)> {
    let id = self.index.get(path).cloned()?;
    let module = self.modules.get(&id).cloned()?;
    Some((id, module))
  }
}

impl Default for ModuleRegistry {
  fn default() -> Self {
    Self::new()
  }
}

pub struct ModuleDescriptor {
  name: Handle<Str>,
  root: Handle<FunctionDescriptor>,
  module_vars: IndexSet<String>,
}

impl ModuleDescriptor {
  pub fn new(
    name: Handle<Str>,
    root: Handle<FunctionDescriptor>,
    module_vars: IndexSet<String>,
  ) -> Self {
    Self {
      name,
      root,
      module_vars,
    }
  }
}

#[derive::delegate_to_handle]
impl ModuleDescriptor {
  pub fn instance(&self, ctx: &Context, module_id: Option<ModuleId>) -> Handle<Module> {
    let name = self.name();
    let root = Function::new(ctx, self.root(), module_id);

    ctx.alloc(Module {
      name,
      root,
      module_vars: Dict::from_iter(
        self
          .module_vars()
          .iter()
          .map(|k| (ctx.alloc(Str::from(k.clone())), Value::none())),
      ),
    })
  }

  pub fn name(&self) -> Handle<Str> {
    self.name.clone()
  }

  pub fn root(&self) -> Handle<FunctionDescriptor> {
    self.root.clone()
  }

  pub(crate) fn module_vars(&self) -> &IndexSet<String> {
    &self.module_vars
  }
}

impl Display for ModuleDescriptor {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<module descriptor {}>", self.name.as_str())
  }
}

impl Access for ModuleDescriptor {}

pub struct Module {
  name: Handle<Str>,
  root: Handle<Function>,
  module_vars: Dict,
}

#[derive::delegate_to_handle]
impl Module {
  pub fn name(&self) -> Handle<Str> {
    self.name.clone()
  }

  pub fn root(&self) -> Handle<Function> {
    self.root.clone()
  }

  pub(crate) unsafe fn module_vars(&self) -> NonNull<Dict> {
    NonNull::from(&self.module_vars)
  }

  pub(crate) unsafe fn module_vars_mut(&mut self) -> NonNull<Dict> {
    NonNull::from(&mut self.module_vars)
  }
}

impl Display for Module {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<module {}>", self.name.as_str())
  }
}

impl Access for Module {
  fn is_frozen(&self) -> bool {
    true
  }

  fn should_bind_methods(&self) -> bool {
    false
  }

  // TODO: tests

  fn field_get(&self, key: &str) -> crate::Result<Option<Value>> {
    Ok(self.module_vars.get(key).cloned())
  }

  fn field_set(&mut self, key: Handle<Str>, value: Value) -> crate::Result<()> {
    let Some(slot) = self.module_vars.get_mut(&key) else {
      return Err(Error::runtime(format!("cannot set field `{key}`")));
    };
    *slot = value;
    Ok(())
  }

  impl_index_via_field!(mut);
}

/// A path composed of segments. Paths are made of alphanumeric ASCII,
/// underscores, and periods.
///
/// Examples:
/// - `module`
/// - `module.nested.another_module`
#[derive(Debug, PartialEq, Eq)]
pub struct Path {
  segments: Vec<String>,
}

impl Path {
  /// # Panics
  /// If `segments` is empty.
  pub fn new(segments: Vec<String>) -> Self {
    assert!(!segments.is_empty());
    Self { segments }
  }
}

impl FromStr for Path {
  type Err = InvalidPathError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    if s.is_empty() {
      return Err(InvalidPathError::EmptySegment { pos: 0 });
    }
    if let Some(pos) = s.find(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '.') {
      return Err(InvalidPathError::InvalidCharacter { pos });
    }

    let mut segments = vec![];
    let mut pos = 0;
    for segment in s.split('.') {
      if segment.is_empty() {
        return Err(InvalidPathError::EmptySegment { pos });
      }
      segments.push(String::from(segment));
      pos += segment.len() + 1;
    }
    Ok(Path { segments })
  }
}

#[derive(Debug, PartialEq, Eq)]
pub enum InvalidPathError {
  EmptySegment { pos: usize },
  InvalidCharacter { pos: usize },
}

impl Display for InvalidPathError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      InvalidPathError::EmptySegment { pos } => write!(f, "empty segment at {pos}"),
      InvalidPathError::InvalidCharacter { pos } => write!(f, "invalid character at {pos}"),
    }
  }
}

impl StdError for InvalidPathError {}

#[derive::delegate_to_handle]
impl Path {
  pub fn segments(&self) -> &[String] {
    &self.segments
  }
}

impl Display for Path {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<path {}>", self.segments().iter().join("."))
  }
}

impl Access for Path {}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_path() {
    assert_eq!(
      Ok(Path {
        segments: vec!["a".into()]
      }),
      Path::from_str("a")
    );
    assert_eq!(
      Ok(Path {
        segments: vec!["a".into(), "b".into()]
      }),
      Path::from_str("a.b")
    );
    assert_eq!(
      Ok(Path {
        segments: vec!["a".into(), "b".into(), "c".into()]
      }),
      Path::from_str("a.b.c")
    );
    assert_eq!(
      Ok(Path {
        segments: vec!["with_underscore".into(), "b".into(), "c".into()]
      }),
      Path::from_str("with_underscore.b.c")
    );
    assert_eq!(
      Err(InvalidPathError::InvalidCharacter { pos: 3 }),
      Path::from_str("bad . bad")
    );
    assert_eq!(
      Err(InvalidPathError::EmptySegment { pos: 0 }),
      Path::from_str("")
    );
    assert_eq!(
      Err(InvalidPathError::EmptySegment { pos: 5 }),
      Path::from_str("test.")
    );
  }
}
