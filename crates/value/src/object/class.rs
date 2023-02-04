use super::func::Params;
use super::handle::Handle;
use super::Dict;
use crate::ptr::{Ref, RefMut};
use crate::Value;

#[derive(Clone)]
pub struct Class {
  pub(super) name: String,
  pub(super) fields: Dict,
  pub(super) parent: Handle<Class>,
}

impl Class {
  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn parent(&self) -> &Handle<Class> {
    &self.parent
  }
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

  pub fn name(&self) -> &str {
    &self.name
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
      d.field(&key, v);
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
