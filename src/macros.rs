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

#[doc(hidden)]
macro_rules! __delegate {
  (
    to($to:expr);
    $( fn $name:ident($self:ident : $self_ty:ty $(, $arg:ident : $ty:ty)*) $(-> $ret:ty)?; )*
  ) => {
    $(
      fn $name($self: $self_ty $(, $arg : $ty)*) $(-> $ret)? {
        let to = $to;
        to.$name($($arg),*)
      }
    )*
  };
}
