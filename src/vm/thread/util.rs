use super::*;
use crate as hebi;

pub fn is_truthy(value: Value) -> bool {
  if value.is_bool() {
    return unsafe { value.to_bool_unchecked() };
  }

  if value.is_float() {
    let value = unsafe { value.to_float_unchecked() };
    return !value.is_nan() && value != 0.0;
  }

  if value.is_int() {
    let value = unsafe { value.to_int_unchecked() };
    return value != 0;
  }

  if value.is_none() {
    return false;
  }

  true
}

pub fn clone_from_raw_slice<T: Clone>(ptr: *mut [T], index: usize) -> T {
  debug_assert!(
    index < std::ptr::metadata(ptr),
    "index out of bounds {index}"
  );
  let value = unsafe { std::mem::ManuallyDrop::new(std::ptr::read((ptr as *mut T).add(index))) };
  std::mem::ManuallyDrop::into_inner(value.clone())
}

pub fn check_args(
  params: &Params,
  has_implicit_receiver: bool,
  num_args: usize,
) -> hebi::Result<()> {
  let has_explicit_self_param = params.has_self && !has_implicit_receiver;

  let min = params.min as usize + has_explicit_self_param as usize;
  let max = params.max as usize + has_explicit_self_param as usize;

  if min > num_args || num_args > max {
    if min == max {
      let plural = if min != 1 { "s" } else { "" };
      hebi::fail!("expected {min} arg{plural}, got {num_args}")
    } else if num_args < min {
      let plural = if min != 1 { "s" } else { "" };
      hebi::fail!("expected at least {min} arg{plural}, got {num_args}")
    } else {
      let plural = if max != 1 { "s" } else { "" };
      hebi::fail!("expected at most {max} arg{plural}, got {num_args}")
    };
  }

  Ok(())
}
