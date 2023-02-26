use super::Isolate;
use crate::value::handle::Handle;
use crate::value::object::{Module, Path};
use crate::value::Value;
use crate::RuntimeError;

// TODO: prevent import cycles by keeping track of which modules are being
// initialized

/// Load module (parse -> emit -> eval root)
pub fn load(vm: &mut Isolate, path: Handle<Path>) -> Result<Handle<Module>, RuntimeError> {
  // path should never be empty
  let name = path.segments().last().unwrap().as_str();

  if let Some((id, module)) = vm.module_registry.by_path(path.segments()) {
    if vm.module_init_visited.contains(&id) {
      return Err(RuntimeError::script(
        format!("attempted to import partially initialized module {name}"),
        0..0,
      ));
    }
    return Ok(module);
  }

  let module_id = vm.module_registry.next_module_id();
  let module = {
    let module = vm
      .module_loader
      .load(path.segments())
      .map_err(|e| RuntimeError::native(e, 0..0))?;
    let module = syntax::parse(module)
      // TODO: propagate parse errors properly
      .map_err(|_| RuntimeError::script(format!("failed to parse module `{name}`"), 0..0))?;
    let module = crate::emit::emit(vm.ctx.clone(), name, &module, false).unwrap();
    println!("{}", module.root().disassemble(true));
    module.instance(&vm.ctx, Some(module_id))
  };
  vm.module_registry
    .add(module_id, path.segments(), module.clone());

  vm.module_init_visited.insert(module_id);

  let result = vm.call(module.root().into(), &[], Value::none());
  if let Err(e) = result {
    // If executing the module root scope results in an error,
    // remove the module from the registry. We do this to ensure
    // that calls to functions declared in this broken module
    // (even in inner scopes) will fail
    vm.module_registry.remove(module_id);
    vm.module_init_visited.remove(&module_id);
    Err(e)?;
  }
  vm.module_init_visited.remove(&module_id);

  Ok(module)
}