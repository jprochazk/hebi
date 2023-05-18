macro_rules! get_raw {
  ($ptr:expr, $field:ident) => {{
    #[allow(unused_unsafe)]
    unsafe {
      ::core::ptr::addr_of!((*$ptr.as_ptr()).$field)
    }
  }};
}

macro_rules! get_raw_mut {
  ($ptr:expr, $field:ident) => {{
    #[allow(unused_unsafe)]
    unsafe {
      ::core::ptr::addr_of_mut!((*$ptr.as_ptr()).$field)
    }
  }};
}

macro_rules! stack_base {
  ($self:ident) => {{
    #[allow(unused_unsafe)]
    unsafe {
      ::core::ptr::read(get_raw!($self.stack, base))
    }
  }};
}

macro_rules! stack_base_mut {
  ($self:ident) => {{
    #[allow(unused_unsafe)]
    unsafe {
      &mut *get_raw_mut!($self.stack, base)
    }
  }};
}

macro_rules! stack {
  ($self:ident) => {{
    #[allow(unused_unsafe)]
    unsafe {
      &$self.stack.as_ref().regs
    }
  }};
}

macro_rules! stack_mut {
  ($self:ident) => {{
    #[allow(unused_unsafe)]
    unsafe {
      &mut $self.stack.as_mut().regs
    }
  }};
}

macro_rules! call_frames {
  ($self:ident) => {{
    #[allow(unused_unsafe)]
    unsafe {
      &*::core::ptr::addr_of!((*$self.stack.as_ptr()).frames)
    }
  }};
}

macro_rules! call_frames_mut {
  ($self:ident) => {{
    #[allow(unused_unsafe)]
    unsafe {
      &mut *::core::ptr::addr_of_mut!((*$self.stack.as_ptr()).frames)
    }
  }};
}

macro_rules! current_call_frame {
  ($self:ident) => {{
    let call_frames = call_frames!($self);
    debug_assert!(!call_frames.is_empty(), "call frame stack is empty");
    unsafe { call_frames.last().unwrap_unchecked() }
  }};
}

macro_rules! current_call_frame_mut {
  ($self:ident) => {{
    let call_frames = call_frames_mut!($self);
    debug_assert!(!call_frames.is_empty(), "call frame stack is empty");
    unsafe { call_frames.last_mut().unwrap_unchecked() }
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
