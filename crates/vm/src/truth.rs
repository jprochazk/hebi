use super::*;

/// This returns `false` if `v` is:
/// - boolean `false`
/// - integer `0`
/// - float `0`
/// - float `NaN`, but specifically not a qnan
///
/// Otherwise, it returns `true`.
pub fn truthiness(v: Value) -> bool {
  if let Some(v) = v.as_bool() {
    v
  } else if let Some(v) = v.as_int() {
    v != 0
  } else if let Some(v) = v.as_float() {
    !v.is_nan() && v != 0.0
  } else {
    true
  }
}
