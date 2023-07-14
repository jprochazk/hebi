use crate::ast::Module;
use crate::gc::{Gc, Ref};
use crate::obj::module::ModuleDescriptor;
use crate::Arena;

pub type Result<T, E = alloc::string::String> = core::result::Result<T, E>;

pub fn module<'arena, 'gc, 'src>(
  arena: &'arena Arena,
  gc: &'gc Gc,
  name: &'src str,
  ast: Module<'arena, 'src>,
) {
  let mut state = State {
    arena,
    gc,
    name,
    ast,
  };
}

struct State<'arena, 'gc, 'src> {
  arena: &'arena Arena,
  gc: &'gc Gc,
  name: &'src str,
  ast: Module<'arena, 'src>,
}

fn top_level<'arena, 'gc, 'src>(
  state: &mut State<'arena, 'gc, 'src>,
) -> Result<Ref<ModuleDescriptor>> {
  todo!()
}
