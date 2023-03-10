use std::fmt::Display;
use std::hash::Hash;

use indexmap::Equivalent;

use super::func::{func_name, Params};
use super::{Access, Dict, Str};
use crate::ctx::Context;
use crate::value::handle::Handle;
use crate::value::Value;
use crate::Result;

// TODO: Display `class def` -> `class` ++ `class` -> `class instance`

pub struct ClassInstance {
  name: Handle<Str>,
  fields: Dict,
  parent: Option<Handle<Class>>,
  is_frozen: bool,
}

#[derive::delegate_to_handle]
impl ClassInstance {
  pub fn name(&self) -> Handle<Str> {
    self.name.clone()
  }

  pub fn parent(&self) -> Option<Handle<Class>> {
    self.parent.clone()
  }

  pub fn has<Q>(&self, key: &Q) -> bool
  where
    Q: ?Sized + Hash + Equivalent<Handle<Str>>,
  {
    self.fields.contains_key(key)
  }

  pub fn get(&self, key: &str) -> Option<&Value> {
    self.fields.get(key)
  }

  pub fn insert(&mut self, key: Handle<Str>, value: Value) -> Option<Value> {
    self.fields.insert(key, value)
  }

  pub fn freeze(&mut self) {
    self.is_frozen = true;
  }
}

impl Access for ClassInstance {
  fn is_frozen(&self) -> bool {
    self.is_frozen
  }

  fn field_get(&self, ctx: &Context, key: &str) -> crate::Result<Option<Value>> {
    Ok(self.fields.get(key).cloned())
  }

  fn field_set(&mut self, ctx: &Context, key: Handle<Str>, value: Value) -> Result<()> {
    self.fields.insert(key, value);
    Ok(())
  }
}

impl Display for ClassInstance {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<class {}>", self.name())
  }
}

/// This type is used to ensure that the `super` keyword always refers to the
/// "syntactical" parent class:
///
/// ```python,ignore
/// class A:
///   fn f(self):
///     print "A"
///
/// class B(A):
///   fn f(self):
///     super.f() # `super` always refers to `A`, even though parent of `C` is `B`.
///     print "B"
///
/// class C(B):
///   fn f(self):
///     super.f() # always refers to `B`
///     print "C"
///
/// C().f() # prints `A B C`
/// ```
pub struct ClassSuperProxy {
  class: Handle<ClassInstance>,
  parent: Handle<Class>,
}

impl ClassSuperProxy {
  pub fn new(class: Handle<ClassInstance>, parent: Handle<Class>) -> Self {
    Self { class, parent }
  }
}

#[derive::delegate_to_handle]
impl ClassSuperProxy {
  pub fn class(&self) -> Handle<ClassInstance> {
    self.class.clone()
  }

  pub fn parent(&self) -> Handle<Class> {
    self.parent.clone()
  }
}

impl Display for ClassSuperProxy {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.parent())
  }
}

impl Access for ClassSuperProxy {
  fn is_frozen(&self) -> bool {
    true
  }

  fn field_get(&self, ctx: &Context, key: &str) -> Result<Option<Value>> {
    self.parent().field_get(ctx, key)
  }
}

pub struct Method {
  this: Value, // ClassInstance or Proxy or NativeClassInstance
  func: Value, // Function or NativeFunction
}

impl Method {
  pub fn new(this: Value, func: Value) -> Self {
    Self { this, func }
  }
}

#[derive::delegate_to_handle]
impl Method {
  pub fn this(&self) -> Value {
    self.this.clone()
  }

  pub fn func(&self) -> Value {
    self.func.clone()
  }
}

impl Display for Method {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<method {}>", func_name(&self.func.clone()))
  }
}

impl Access for Method {}

pub struct Class {
  desc: Handle<ClassDescriptor>,
  methods: Dict,
  fields: Dict,
  parent: Option<Handle<Class>>,
}

impl Class {
  pub fn new(ctx: Context, desc: Handle<ClassDescriptor>, args: &[Value]) -> Self {
    assert!(args.len() >= desc.is_derived() as usize + desc.methods().len() + desc.fields().len());

    let parent_offset = 0;
    let methods_offset = parent_offset + desc.is_derived() as usize;
    let fields_offset = methods_offset + desc.methods().len();

    let parent = desc
      .is_derived()
      .then(|| args[parent_offset].clone().to_object::<Class>().unwrap());

    let mut methods = Dict::with_capacity(desc.methods().len());
    for (k, v) in desc.methods().iter().zip(args[methods_offset..].iter()) {
      methods.insert(ctx.alloc(k.clone()), v.clone());
    }

    let mut fields = Dict::with_capacity(desc.fields().len());
    for (k, v) in desc.fields().iter().zip(args[fields_offset..].iter()) {
      fields.insert(ctx.alloc(k.clone()), v.clone());
    }

    // inherit methods and field defaults
    if let Some(parent) = &parent {
      for (k, v) in parent.methods().iter() {
        methods.entry(k.clone()).or_insert_with(|| v.clone());
      }
      for (k, v) in parent.fields().iter() {
        fields.entry(k.clone()).or_insert_with(|| v.clone());
      }
    }

    Self {
      desc,
      methods,
      fields,
      parent,
    }
  }
}

#[derive::delegate_to_handle]
impl Class {
  pub fn instance(&self) -> ClassInstance {
    let name = self.name();
    let parent = self.parent.clone();

    let mut fields = Dict::with_capacity(self.fields.len() + self.methods.len());
    for (k, v) in self.fields.iter().chain(self.methods.iter()) {
      fields.insert(k.clone(), v.clone());
    }

    ClassInstance {
      name,
      fields,
      parent,
      is_frozen: false,
    }
  }

  pub fn name(&self) -> Handle<Str> {
    self.desc.name()
  }

  pub fn method(&self, key: &str) -> Option<Value> {
    self.methods.get(key).cloned()
  }

  pub fn parent(&self) -> Option<Handle<Class>> {
    self.parent.clone()
  }

  pub fn methods(&self) -> &Dict {
    &self.methods
  }

  pub fn fields(&self) -> &Dict {
    &self.fields
  }

  pub fn params(&self) -> &Params {
    self.desc.params()
  }
}

impl Display for Class {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<class def {}>", self.name())
  }
}

impl Access for Class {
  fn should_bind_methods(&self) -> bool {
    false
  }

  fn field_get(&self, ctx: &Context, key: &str) -> Result<Option<Value>> {
    Ok(self.method(key))
  }
}

pub struct ClassDescriptor {
  name: Handle<Str>,
  params: Params,
  is_derived: bool,
  methods: Vec<Str>,
  fields: Vec<Str>,
}

impl ClassDescriptor {
  pub fn new(
    name: Handle<Str>,
    params: Params,
    is_derived: bool,
    methods: Vec<Str>,
    fields: Vec<Str>,
  ) -> Self {
    Self {
      name,
      params,
      is_derived,
      methods,
      fields,
    }
  }
}

#[derive::delegate_to_handle]
impl ClassDescriptor {
  pub fn name(&self) -> Handle<Str> {
    self.name.clone()
  }

  pub fn params(&self) -> &Params {
    &self.params
  }

  pub fn is_derived(&self) -> bool {
    self.is_derived
  }

  pub fn methods(&self) -> &[Str] {
    &self.methods
  }

  pub fn fields(&self) -> &[Str] {
    &self.fields
  }
}

impl Display for ClassDescriptor {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<class descriptor {}>", self.name())
  }
}

impl Access for ClassDescriptor {}
