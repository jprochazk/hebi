use std::hash::Hash;

use indexmap::Equivalent;

use super::dict::Key;
use super::func::Params;
use super::handle::Handle;
use super::Dict;
use crate::ptr::Ref;
use crate::Value;

#[derive(Clone)]
pub struct Class {
  pub(super) name: String,
  pub(super) fields: Dict,
  pub(super) parent: Option<Handle<ClassDef>>,
  is_frozen: bool,
}

impl Class {
  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn parent(&self) -> Option<&Handle<ClassDef>> {
    self.parent.as_ref()
  }

  pub fn has<Q>(&self, key: &Q) -> bool
  where
    Q: ?Sized + Hash + Equivalent<Key>,
  {
    self.fields.contains_key(key)
  }

  pub fn get<Q>(&self, key: &Q) -> Option<&Value>
  where
    Q: ?Sized + Hash + Equivalent<Key>,
  {
    self.fields.get(key)
  }

  pub fn set<Q>(&mut self, key: &Q, value: Value) -> bool
  where
    Q: ?Sized + Hash + Equivalent<Key>,
  {
    let Some(slot) = self.fields.get_mut(key) else {
      return false;
    };
    *slot = value;
    true
  }

  pub fn insert(&mut self, key: Key, value: Value) -> Option<Value> {
    self.fields.insert(key, value)
  }

  pub fn remove<Q>(&mut self, key: &Q) -> Option<Value>
  where
    Q: ?Sized + Hash + Equivalent<Key>,
  {
    self.fields.remove(key)
  }

  pub fn is_frozen(&self) -> bool {
    self.is_frozen
  }

  pub fn freeze(&mut self) {
    self.is_frozen = true;
  }
}

#[derive(Clone, Debug)]
pub struct Method {
  pub this: Handle<Class>,
  pub func: Value, // Func or Closure
}

// TODO: Shape

#[derive(Clone)]
pub struct ClassDef {
  pub(super) name: String,
  pub(super) params: Params,
  pub(super) methods: Dict,
  pub(super) fields: Dict,
  pub(super) parent: Option<Handle<ClassDef>>,
}

impl ClassDef {
  pub fn new(descriptor: Handle<ClassDesc>, args: &[Value]) -> Self {
    let descriptor = descriptor.borrow();
    assert!(
      args.len()
        >= descriptor.is_derived as usize + descriptor.methods.len() + descriptor.fields.len()
    );

    let name = descriptor.name.clone();
    let params = descriptor.params.clone();

    let parent_offset = 0;
    let methods_offset = parent_offset + descriptor.is_derived as usize;
    let fields_offset = methods_offset + descriptor.methods.len();

    let parent = descriptor
      .is_derived
      .then(|| Handle::from_value(args[parent_offset].clone()).unwrap());

    // TODO: inherit non-overridden methods from parent by copying them

    let mut methods = Dict::with_capacity(descriptor.methods.len());
    for (k, v) in descriptor.methods.iter().zip(args[methods_offset..].iter()) {
      methods.insert(k.clone().into(), v.clone());
    }

    let mut fields = Dict::with_capacity(descriptor.fields.len());
    for (k, v) in descriptor.fields.iter().zip(args[fields_offset..].iter()) {
      fields.insert(k.clone().into(), v.clone());
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
    let mut class = unsafe {
      Handle::<Class>::from_value(
        Class {
          name,
          fields: Dict::with_capacity(self.methods.len() + self.fields.len()),
          parent,
          is_frozen: false,
        }
        .into(),
      )
      .unwrap_unchecked()
    };

    for (k, v) in self.methods.iter() {
      let v = Method {
        this: class.clone(),
        func: v.clone(),
      }
      .into();
      class.borrow_mut().fields.insert(k.clone(), v);
    }
    for (k, v) in self.fields.iter() {
      dbg!(&v);
      class.borrow_mut().fields.insert(k.clone(), v.clone());
    }

    class
  }

  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn params(&self) -> &Params {
    &self.params
  }

  pub fn method<Q>(&self, key: &Q) -> Option<&Value>
  where
    Q: ?Sized + Hash + Equivalent<Key>,
  {
    self.methods.get(key)
  }
}

#[derive(Clone, Debug)]
pub struct ClassDesc {
  pub name: String,
  pub params: Params,
  pub is_derived: bool,
  pub methods: Vec<String>,
  pub fields: Vec<String>,
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
      .field(
        "parent",
        &self
          .parent
          .as_ref()
          .map(|p| Ref::map(p.borrow(), |v| v.name())),
      )
      .finish()
  }
}
