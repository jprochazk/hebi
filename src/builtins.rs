use crate::public::conv::IntoHebi;
use crate::public::{Context, Dict, Value};
use crate::value::object::native;
use crate::{Error, Hebi, Result};

fn str<'a>(ctx: &'a Context<'a>, args: &'a [Value<'a>], _: Option<Dict<'a>>) -> Result<Value<'a>> {
  if args.len() != 1 {
    return Err(Error::runtime(format!(
      "expected exactly 1 argument, got {}",
      args.len()
    )));
  }

  let value = args[0].clone();
  format!("{value}").into_hebi(ctx)
}

fn r#type<'a>(
  ctx: &'a Context<'a>,
  args: &'a [Value<'a>],
  _: Option<Dict<'a>>,
) -> Result<Value<'a>> {
  if args.len() != 1 {
    return Err(Error::runtime(format!(
      "expected exactly 1 argument, got {}",
      args.len()
    )));
  }

  let value = args[0].clone();
  let ty = if value.is_float() {
    "float"
  } else if value.is_int() {
    "int"
  } else if value.is_bool() {
    "bool"
  } else if value.is_none() {
    "none"
  } else {
    // TODO: type name trait, or something
    "object"
  };

  ty.into_hebi(ctx)
}

pub fn register(hebi: &Hebi) {
  hebi.globals().register_fn("str", str);
  hebi.globals().register_fn("type", r#type);
  hebi.globals().register_class::<native::Test>();
}
