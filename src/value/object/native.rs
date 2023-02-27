trait Callable {
  fn call(vm: &Hebi, args: &[Value<'_>], kwargs: Value<'_>) -> Result<Value<'_>, Error>;
}

macro_rules! impl_callable_for_fn {
  () => {
    impl<'a, F, R, $($T),*> Callable for F
    where
      F: FnOnce($($T),*) -> R,
      F: Clone + Send + 'static,
      R: IntoHebi<'a>,
      $($T : FromHebi<'a>,)*
    {
      fn call(vm: &Hebi, args: &[Value<'_>], )
    }
  }
}

/*

import my_module

my_module.yep_clock(now(), option=true)


*/
