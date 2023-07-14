#![allow(clippy::needless_lifetimes)]

mod builder;

use core::hash::BuildHasherDefault;

use bumpalo::collections::Vec;
use bumpalo::vec;
use rustc_hash::FxHasher;

use crate::ast::Module;
use crate::gc::{Gc, Ref};
use crate::obj::module::ModuleDescriptor;
use crate::obj::string::Str;
use crate::Arena;

use self::builder::BytecodeBuilder;

use super::MVar;

type HashSet<T, A> = hashbrown::HashSet<T, BuildHasherDefault<FxHasher>, A>;
type HashMap<K, V, A> = hashbrown::HashMap<K, V, BuildHasherDefault<FxHasher>, A>;

pub type Result<T, E = alloc::string::String> = core::result::Result<T, E>;

pub fn module<'arena, 'gc, 'src>(
  arena: &'arena Arena,
  gc: &'gc Gc,
  name: &'src str,
  ast: Module<'arena, 'src>,
) -> Result<Ref<ModuleDescriptor>> {
  let mut state = ModuleState {
    arena,
    gc,
    name,
    ast,

    module_vars: HashSet::with_hasher_in(BuildHasherDefault::default(), arena),
  };

  let mut state = FunctionState {
    module: &mut state,
    parent: None,
    arena,
    gc,
    name,
    builder: BytecodeBuilder::new_in(arena),
    registers: 0,
  };

  top_level(&mut state)
}

struct ModuleState<'arena, 'gc, 'src> {
  arena: &'arena Arena,
  gc: &'gc Gc,
  name: &'src str,
  ast: Module<'arena, 'src>,

  module_vars: HashSet<(Ref<Str>, MVar<usize>), &'arena Arena>,
}

struct FunctionState<'arena, 'gc, 'src, 'state> {
  module: &'state mut ModuleState<'arena, 'gc, 'src>,
  parent: Option<&'state mut FunctionState<'arena, 'gc, 'src, 'state>>,

  arena: &'arena Arena,
  gc: &'gc Gc,
  name: &'src str,

  builder: BytecodeBuilder<'arena>,
  registers: usize,
}

fn top_level(state: &mut FunctionState) -> Result<Ref<ModuleDescriptor>> {
  todo!()
}
