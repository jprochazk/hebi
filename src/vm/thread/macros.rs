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

macro_rules! push_args {
  ($self:ident, $callee:expr, range($start:expr, $end:expr)) => {{
    let callee = $callee;
    let start = $start;
    let end = $end;
    let stack_base = $self.stack.len();
    let num_args = end - start;
    $self.stack.push(callee);
    $self.stack.extend_from_within(start..end);
    (stack_base, num_args)
  }};
  ($self:ident, $callee:expr, $args:expr) => {{
    let callee = $callee;
    let args = $args;
    let stack_base = $self.stack.len();
    let num_args = args.len();
    $self.stack.push(callee);
    $self.stack.extend_from_slice(args);
    (stack_base, num_args)
  }};
  ($self:ident, $args:expr) => {{
    let args = $args;
    let stack_base = $self.stack.len();
    let num_args = args.len();
    $self.stack.extend_from_slice(args);
    (stack_base, num_args)
  }};
}

macro_rules! debug_assert_object_type {
  ($value:ident, $ty:ty) => {{
    let value = match $value.clone().to_object() {
      Some(value) => value,
      None => panic!("{} is not an object", stringify!($value)),
    };
    if let Err(e) = value.cast::<$ty>() {
      panic!("{e} is not {}", ::std::any::type_name::<$ty>());
    }
  }};
}
