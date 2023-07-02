use std::borrow::Borrow;
use std::cell::Cell;
use std::cmp::Ordering;
use std::fmt::{Debug, Display};
use std::ops::Deref;

use super::builtin::BuiltinMethod;
use super::{Object, Ptr};
use crate::internal::error::Result;
use crate::internal::value::Value;
use crate::internal::vm::global::Global;
use crate::public::Scope;
use crate::Cow;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Str {
  data: Cow<'static, str>,
}

impl Str {
  #[allow(dead_code)] // symmetry with `owned`
  pub fn borrowed(data: &'static str) -> Self {
    Self {
      data: Cow::borrowed(data),
    }
  }

  pub fn owned(data: impl ToString) -> Self {
    Self {
      data: Cow::owned(data.to_string()),
    }
  }

  pub fn as_str(&self) -> &str {
    self.data.as_ref()
  }

  pub fn concat(&self, other: &str) -> Self {
    let mut out = String::with_capacity(self.len() + other.len());
    out.push_str(self.as_str());
    out.push_str(other);
    Self::owned(out)
  }
}

fn str_len(this: Ptr<Str>, _: Scope<'_>) -> Result<Value> {
  Ok(Value::int(this.len() as i32))
}

fn str_is_empty(this: Ptr<Str>, _: Scope<'_>) -> Result<Value> {
  Ok(Value::bool(this.is_empty()))
}

pub struct LinesIter {
  str: Ptr<Str>,
  offset: Cell<Option<usize>>,
  done: Cell<bool>,
}

impl Display for LinesIter {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "<str lines>")
  }
}

impl Debug for LinesIter {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("LinesIter")
      .field("str", &self.str)
      .field("offset", &self.offset)
      .finish()
  }
}

impl Object for LinesIter {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "LinesIter"
  }

  default_instance_of!();

  fn named_field(scope: Scope<'_>, this: Ptr<Self>, name: Ptr<Str>) -> Result<Value> {
    Ok(
      this
        .named_field_opt(scope, name.clone())?
        .ok_or_else(|| error!("`{this}` has no field `{name}`"))?,
    )
  }

  fn named_field_opt(
    scope: Scope<'_>,
    this: Ptr<Self>,
    name: Ptr<super::Str>,
  ) -> Result<Option<Value>> {
    let method = match name.as_str() {
      "iter" => builtin_method!(str_lines_iter),
      "next" => builtin_method!(str_lines_next),
      "done" => builtin_method!(str_lines_done),
      _ => fail!("`{this}` has no field `{name}`"),
    };

    Ok(Some(Value::object(unsafe {
      scope.alloc(BuiltinMethod::new(Value::object(this), method))
    })))
  }
}

declare_object_type!(LinesIter);

fn str_lines_iter(this: Ptr<LinesIter>, _: Scope<'_>) -> Result<Value> {
  Ok(Value::object(this))
}

fn str_lines_next(this: Ptr<LinesIter>, scope: Scope<'_>) -> Result<Value> {
  if this.done.get() {
    return Ok(Value::none());
  }

  if let Some(offset) = this.offset.get() {
    let start: usize = offset + 1;
    let str = match this.str.as_str()[start..].find('\n') {
      Some(end) => {
        let end = start + end;
        this.offset.set(Some(end));
        scope.alloc(Str::owned(&this.str.as_str()[start..end]))
      }
      None => {
        this.done.set(true);
        scope.alloc(Str::owned(&this.str.as_str()[start..]))
      }
    };
    Ok(Value::object(str))
  } else {
    let str = match this.str.as_str().find('\n') {
      Some(end) => {
        this.offset.set(Some(end));
        scope.alloc(Str::owned(&this.str.as_str()[..end]))
      }
      None => {
        this.done.set(true);
        this.str.clone()
      }
    };
    Ok(Value::object(str))
  }
}

fn str_lines_done(this: Ptr<LinesIter>, _: Scope<'_>) -> Result<Value> {
  Ok(Value::bool(this.done.get()))
}

fn str_lines(this: Ptr<Str>, scope: Scope<'_>) -> Result<Value> {
  Ok(Value::object(scope.alloc(LinesIter {
    str: this,
    offset: Cell::new(None),
    done: Cell::new(false),
  })))
}

impl Object for Str {
  fn type_name(_: Ptr<Self>) -> &'static str {
    "String"
  }

  default_instance_of!();

  fn named_field(scope: Scope<'_>, this: Ptr<Self>, name: Ptr<Str>) -> Result<Value> {
    Ok(
      this
        .named_field_opt(scope, name.clone())?
        .ok_or_else(|| error!("`{this}` has no field `{name}`"))?,
    )
  }

  fn named_field_opt(
    scope: Scope<'_>,
    this: Ptr<Self>,
    name: Ptr<super::Str>,
  ) -> Result<Option<Value>> {
    let method = match name.as_str() {
      "len" => builtin_method!(str_len),
      "is_empty" => builtin_method!(str_is_empty),
      "lines" => builtin_method!(str_lines),
      _ => fail!("`{this}` has no field `{name}`"),
    };

    Ok(Some(Value::object(unsafe {
      scope.alloc(BuiltinMethod::new(Value::object(this), method))
    })))
  }

  fn add(scope: Scope<'_>, this: Ptr<Self>, other: Ptr<Self>) -> Result<Value> {
    Ok(Value::object(scope.alloc(this.concat(other.as_str()))))
  }

  fn cmp(_: Scope<'_>, this: Ptr<Self>, other: Ptr<Self>) -> Result<Ordering> {
    Ok(this.as_str().cmp(other.as_str()))
  }
}

pub fn register_builtin_functions(global: &Global) {
  bind_builtin_type!(
    global,
    builtin_type!(Str {
      len: builtin_method_static!(Str, str_len),
      is_empty: builtin_method_static!(Str, str_is_empty),
      lines: builtin_method_static!(Str, str_lines)
    })
  );
}

declare_object_type!(Str);

impl Display for Str {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Display::fmt(&self.data, f)
  }
}

impl Debug for Str {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    Debug::fmt(&self.data, f)
  }
}

impl Deref for Str {
  type Target = str;

  fn deref(&self) -> &Self::Target {
    self.data.as_ref()
  }
}

impl std::borrow::Borrow<str> for Str {
  fn borrow(&self) -> &str {
    self.data.borrow()
  }
}

impl AsRef<str> for Str {
  fn as_ref(&self) -> &str {
    self.data.as_ref()
  }
}

impl indexmap::Equivalent<str> for Ptr<Str> {
  fn equivalent(&self, key: &str) -> bool {
    self.as_str() == key
  }
}

impl Borrow<str> for Ptr<Str> {
  fn borrow(&self) -> &str {
    self
  }
}

impl<'a> PartialEq<&'a str> for Ptr<Str> {
  fn eq(&self, other: &&'a str) -> bool {
    self.as_str() == *other
  }
}
