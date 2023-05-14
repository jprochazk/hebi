// TODO: cache current call frame in a field (Option<T>),
// so that it's one less indirection access

macro_rules! current_call_frame {
  ($self:ident) => {{
    debug_assert!(
      !$self.call_frames.borrow().is_empty(),
      "call frame stack is empty"
    );
    ::std::cell::Ref::map($self.call_frames.borrow(), |frames| unsafe {
      frames.last().unwrap_unchecked()
    })
  }};
}

macro_rules! current_call_frame_mut {
  ($self:ident) => {{
    debug_assert!(
      !$self.call_frames.borrow().is_empty(),
      "call frame stack is empty"
    );
    ::std::cell::RefMut::map($self.call_frames.borrow_mut(), |frames| unsafe {
      frames.last_mut().unwrap_unchecked()
    })
  }};
}

macro_rules! binary {
  ($lhs:ident, $rhs:ident {
    i32 => $i32_expr:expr,
    f64 => $f64_expr:expr,
    any => $any_expr:expr,
  }) => {{
    if $lhs.is_int() && $rhs.is_int() {
      let $lhs = unsafe { $lhs.to_int_unchecked() };
      let $rhs = unsafe { $rhs.to_int_unchecked() };
      $i32_expr
    } else if $lhs.is_float() && $rhs.is_float() {
      let $lhs = unsafe { $lhs.to_float_unchecked() };
      let $rhs = unsafe { $rhs.to_float_unchecked() };
      $f64_expr
    } else if $lhs.is_float() && $rhs.is_int() {
      let $lhs = unsafe { $lhs.to_float_unchecked() };
      let $rhs = unsafe { $rhs.to_int_unchecked() } as f64;
      $f64_expr
    } else if $lhs.is_int() && $rhs.is_float() {
      let $lhs = unsafe { $lhs.to_int_unchecked() } as f64;
      let $rhs = unsafe { $rhs.to_float_unchecked() };
      $f64_expr
    } else if $lhs.is_bool() && $rhs.is_bool() {
      hebi::fail!("cannot {} `bool`", stringify!($op))
    } else if $lhs.is_none() && $rhs.is_none() {
      hebi::fail!("cannot {} `none`", stringify!($op))
    } else if $lhs.is_object() && $rhs.is_object() {
      let $lhs = unsafe { $lhs.to_any_unchecked() };
      let $rhs = unsafe { $rhs.to_any_unchecked() };
      if $lhs.ty() != $rhs.ty() {
        hebi::fail!("operands must have the same type: `{}`, `{}`", $lhs, $rhs)
      }
      $any_expr
    } else {
      hebi::fail!("operands must have the same type: `{}`, `{}`", $lhs, $rhs)
    }
  }};
}
