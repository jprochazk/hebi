use super::string::Str;
use crate::gc::Ref;

pub struct ModuleDescriptor {
  pub name: Ref<Str>,
}
