#[macro_export]
macro_rules! error {
  ($fmt:literal $(,$($arg:tt)*)?) => {
    $crate::span::SpannedError::new(format!($fmt $(, $($arg)*)?), None)
  };
  ($msg:expr) => {
    $crate::span::SpannedError::new($msg, None)
  };
  (@$span:expr, $fmt:literal $(,$($arg:tt)*)?) => {
    $crate::span::SpannedError::new(format!($fmt $(, $($arg)*)?), $span)
  };
  (@$span:expr, $msg:expr) => {
    $crate::span::SpannedError::new($msg, $span)
  };
}

#[macro_export]
macro_rules! fail {
  ($fmt:literal $(,$($arg:tt)*)?) => {
    return Err(error!($fmt $(,$($arg)*)?).into())
  };
  ($msg:expr) => {
    return Err(error!($msg).into())
  };
  (@$span:expr, $fmt:literal $(,$($arg:tt)*)?) => {
    return Err(error!(@$span, $fmt $(, $($arg)*)?).into())
  };
  (@$span:expr, $msg:expr) => {
    return Err(error!(@$span, $msg).into())
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
