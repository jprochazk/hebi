use crate::public::conv::IntoHebi;
use crate::public::{Context, Dict, Value};
use crate::{Hebi, RuntimeError};

fn str<'a>(
  ctx: &'a Context<'a>,
  args: &'a [Value<'a>],
  _: &'a Dict<'a>,
) -> Result<Value<'a>, RuntimeError> {
  if args.len() != 1 {
    return Err(RuntimeError::script(
      format!("expected exactly 1 argument, got {}", args.len()),
      0..0,
    ));
  }

  let value = args[0].clone();
  format!("{value}").into_hebi(ctx)
}

fn r#type<'a>(
  ctx: &'a Context<'a>,
  args: &'a [Value<'a>],
  _: &'a Dict<'a>,
) -> Result<Value<'a>, RuntimeError> {
  if args.len() != 1 {
    return Err(RuntimeError::script(
      format!("expected exactly 1 argument, got {}", args.len()),
      0..0,
    ));
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
  hebi.globals().set("str", hebi.create_function(str));
  hebi.globals().set("type", hebi.create_function(r#type));
}
