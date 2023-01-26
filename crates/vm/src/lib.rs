use value::object;

pub struct Isolate {
  // TODO: module registry
  globals: object::Dict,
}
