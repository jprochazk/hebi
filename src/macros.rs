#[macro_export]
#[doc(hidden)]
macro_rules! fail {
  ($fmt:literal $(,$($arg:tt)*)?) => {
    return Err($crate::span::SpannedError::new(format!($fmt $(, $($arg)*)?), None).into())
  };
  ($msg:expr) => {
    return Err($crate::span::SpannedError::new($msg, None).into())
  };
  (@$span:expr, $fmt:literal $(,$($arg:tt)*)?) => {
    return Err($crate::span::SpannedError::new(format!($fmt $(, $($arg)*)?), $span).into())
  };
  (@$span:expr, $msg:expr) => {
    return Err($crate::span::SpannedError::new($msg, $span).into())
  };
}
