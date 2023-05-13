use super::*;

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

pub fn check_args(params: &Params, n: usize) -> HebiResult<()> {
  if !params.matches(n) {
    let min = params.min as usize;
    let max = params.max as usize;
    let msg = if min == max {
      let plural = if min != 1 { "s" } else { "" };
      format!("expected {min} arg{plural}, got {n}")
    } else if n < min {
      let plural = if min != 1 { "s" } else { "" };
      format!("expected at least {min} arg{plural}, got {n}")
    } else {
      let plural = if max != 1 { "s" } else { "" };
      format!("expected at most {max} arg{plural}, got {n}")
    };
    fail!(msg);
  }
  Ok(())
}
