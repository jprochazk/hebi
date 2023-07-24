use core::fmt::{Debug, Display};

use super::func::FunctionDescriptor;
use super::string::Str;
use crate::error::AllocError;
use crate::gc::{Gc, Object, Ref};

#[derive(Debug)]
pub struct ModuleDescriptor {
  name: Ref<Str>,
  root: Ref<FunctionDescriptor>,
  num_vars: u16,
}

impl ModuleDescriptor {
  pub fn try_new_in(
    gc: &Gc,
    name: &str,
    root: Ref<FunctionDescriptor>,
    num_vars: u16,
  ) -> Result<Ref<Self>, AllocError> {
    let name = Str::try_new_in(gc, name)?;
    gc.try_alloc(ModuleDescriptor {
      name,
      root,
      num_vars,
    })
  }

  pub fn name(&self) -> Ref<Str> {
    self.name
  }

  pub fn root(&self) -> Ref<FunctionDescriptor> {
    self.root
  }

  pub fn num_vars(&self) -> u16 {
    self.num_vars
  }
}

impl Display for ModuleDescriptor {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "<module `{}`>", self.name)
  }
}

impl Object for ModuleDescriptor {}
