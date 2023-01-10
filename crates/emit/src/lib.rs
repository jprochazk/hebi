use std::hash::Hash;

use beef::lean::Cow;
use syntax::ast;

struct Emitter<'src, Value: Hash + Eq> {
  state: State<'src, Value>,
  module: &'src ast::Module<'src>,
}

struct State<'src, Value: Hash + Eq> {
  name: Cow<'src, str>,
  builder: op::BytecodeBuilder<Value>,
  parent: Option<Box<State<'src, Value>>>,
}
