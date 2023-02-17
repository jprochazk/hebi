use std::fmt::Debug;
use std::hash::Hash;

use indexmap::Equivalent;

use super::dict::{Key, StaticKey};
use super::func::Params;
use super::handle::Handle;
use super::{Access, Dict, Str};
use crate::value::Value;

#[derive(Clone)]
pub struct Class {
  pub(super) name: Str,
  pub(super) fields: Dict,
  pub(super) parent: Option<Handle<ClassDef>>,
  is_frozen: bool,
}

#[derive(Clone)]
pub struct Proxy {
  pub(super) class: Handle<Class>,
  pub(super) parent: Handle<ClassDef>,
}

impl Proxy {
  pub fn new(class: Handle<Class>, parent: Handle<ClassDef>) -> Handle<Self> {
    Self { class, parent }.into()
  }

  pub fn class(&self) -> &Handle<Class> {
    &self.class
  }

  pub fn parent(&self) -> &Handle<ClassDef> {
    &self.parent
  }
}

impl Class {
  pub fn name(&self) -> &str {
    self.name.as_str()
  }

  pub fn parent(&self) -> Option<&Handle<ClassDef>> {
    self.parent.as_ref()
  }

  pub fn has<Q>(&self, key: &Q) -> bool
  where
    Q: ?Sized + Hash + Equivalent<StaticKey>,
  {
    self.fields.contains_key(key)
  }

  pub fn get(&self, key: impl Into<StaticKey>) -> Option<&Value> {
    let key = key.into();
    self.fields.get(&key)
  }

  pub fn insert(&mut self, key: impl Into<StaticKey>, value: Value) -> Option<Value> {
    self.fields.insert(key, value)
  }

  pub fn remove<Q>(&mut self, key: &Q) -> Option<Value>
  where
    Q: ?Sized + Hash + Equivalent<StaticKey>,
  {
    self.fields.remove(key)
  }

  pub fn freeze(&mut self) {
    self.is_frozen = true;
  }
}

impl Access for Class {
  fn is_frozen(&self) -> bool {
    self.is_frozen
  }

  fn field_get(&self, key: &Key<'_>) -> Result<Option<Value>, crate::Error> {
    Ok(match key {
      Key::Int(v) => self.fields.get(v).cloned(),
      Key::Str(v) => self.fields.get(v.as_str()).cloned(),
      Key::Ref(v) => self.fields.get(*v).cloned(),
    })
  }

  fn field_set(&mut self, key: StaticKey, value: Value) -> Result<(), crate::Error> {
    self.fields.insert(key, value);
    Ok(())
  }
}

#[derive(Clone, Debug)]
pub struct Method {
  pub this: Value, // Class or Proxy
  pub func: Value, // Func or Closure
}

// TODO: Shape

#[derive(Clone)]
pub struct ClassDef {
  pub(super) name: Str,
  pub(super) params: Params,
  pub(super) methods: Dict,
  pub(super) fields: Dict,
  pub(super) parent: Option<Handle<ClassDef>>,
}

impl ClassDef {
  pub fn new(desc: Handle<ClassDesc>, args: &[Value]) -> Self {
    assert!(args.len() >= desc.is_derived as usize + desc.methods.len() + desc.fields.len());

    let name = desc.name.clone();
    let params = desc.params.clone();

    let parent_offset = 0;
    let methods_offset = parent_offset + desc.is_derived as usize;
    let fields_offset = methods_offset + desc.methods.len();

    let parent = desc
      .is_derived
      .then(|| Handle::<ClassDef>::from_value(args[parent_offset].clone()).unwrap());

    let mut methods = Dict::with_capacity(desc.methods.len());
    for (k, v) in desc.methods.iter().zip(args[methods_offset..].iter()) {
      methods.insert(k.clone(), v.clone());
    }

    let mut fields = Dict::with_capacity(desc.fields.len());
    for (k, v) in desc.fields.iter().zip(args[fields_offset..].iter()) {
      fields.insert(k.clone(), v.clone());
    }

    // inherit methods and field defaults
    if let Some(parent) = &parent {
      for (k, v) in parent.methods.iter() {
        methods.entry(k.clone()).or_insert_with(|| v.clone());
      }
      for (k, v) in parent.fields.iter() {
        fields.entry(k.clone()).or_insert_with(|| v.clone());
      }
    }

    Self {
      name,
      params,
      methods,
      fields,
      parent,
    }
  }

  pub fn instance(&self) -> Handle<Class> {
    let name = self.name.clone();
    let parent = self.parent.clone();

    let mut fields = Dict::with_capacity(self.fields.len() + self.methods.len());
    for (k, v) in self.fields.iter().chain(self.methods.iter()) {
      fields.insert(k.clone(), v.clone());
    }

    Class {
      name,
      fields,
      parent,
      is_frozen: false,
    }
    .into()
  }

  pub fn name(&self) -> &str {
    self.name.as_str()
  }

  pub fn params(&self) -> &Params {
    &self.params
  }

  pub fn method(&self, key: &str) -> Option<Value> {
    self.methods.get(key).cloned()
  }

  pub fn parent(&self) -> Option<&Handle<ClassDef>> {
    self.parent.as_ref()
  }
}

impl Access for ClassDef {
  fn should_bind_methods(&self) -> bool {
    false
  }

  fn field_get(&self, key: &Key<'_>) -> Result<Option<Value>, crate::Error> {
    Ok(match key {
      Key::Int(_) => None,
      Key::Str(v) => self.method(v.as_str()),
      Key::Ref(v) => self.method(v),
    })
  }
}

#[derive(Clone, Debug)]
pub struct ClassDesc {
  pub name: Str,
  pub params: Params,
  pub is_derived: bool,
  pub methods: Vec<Str>,
  pub fields: Vec<Str>,
}

impl std::fmt::Debug for Class {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let mut d = f.debug_struct("Class");

    let mut key = String::new();
    for (k, v) in self.fields.iter() {
      key.clear();
      k.write_to_string(&mut key);
      if v.is_method() {
        d.field(&key, &v.as_method().unwrap().func);
      } else {
        d.field(&key, v);
      }
    }

    d.finish()
  }
}

impl std::fmt::Debug for ClassDef {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ClassDef")
      .field("defaults", &self.fields)
      .field("methods", &self.methods)
      .field("parent", &self.parent.as_ref().map(|p| p.name()))
      .finish()
  }
}

impl std::fmt::Debug for Proxy {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    std::fmt::Debug::fmt(&self.parent, f)
  }
}

impl Access for Proxy {
  fn is_frozen(&self) -> bool {
    true
  }

  fn field_get(&self, key: &Key<'_>) -> Result<Option<Value>, crate::Error> {
    self.parent().field_get(key)
  }
}

impl Access for Method {}
impl Access for ClassDesc {}
