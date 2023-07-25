use core::fmt::{Debug, Display};

use super::func::FunctionProto;
use super::string::Str;
use crate::error::AllocError;
use crate::gc::{Gc, Object, Ref};

#[derive(Debug)]
pub struct ModuleProto {
  name: Ref<Str>,
  root: Ref<FunctionProto>,
  num_vars: u16,
}

impl ModuleProto {
  pub fn try_new_in(
    gc: &Gc,
    name: &str,
    root: Ref<FunctionProto>,
    num_vars: u16,
  ) -> Result<Ref<Self>, AllocError> {
    let name = Str::try_new_in(gc, name)?;
    gc.try_alloc(ModuleProto {
      name,
      root,
      num_vars,
    })
  }

  pub fn name(&self) -> Ref<Str> {
    self.name
  }

  pub fn root(&self) -> Ref<FunctionProto> {
    self.root
  }

  pub fn num_vars(&self) -> u16 {
    self.num_vars
  }
}

impl Display for ModuleProto {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "<module `{}`>", self.name)
  }
}

impl Object for ModuleProto {}
