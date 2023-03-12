use proc_macro::TokenStream;

mod class;
mod delegate;
mod function;
mod util;

#[doc(hidden)]
#[proc_macro_attribute]
pub fn delegate_to_handle(args: TokenStream, input: TokenStream) -> TokenStream {
  delegate::macro_impl(args, input)
}

#[proc_macro_attribute]
pub fn function(args: TokenStream, input: TokenStream) -> TokenStream {
  function::macro_impl(args, input)
}

#[proc_macro_attribute]
pub fn class(args: TokenStream, input: TokenStream) -> TokenStream {
  class::class_macro_impl(args, input)
}

#[proc_macro_attribute]
pub fn methods(args: TokenStream, input: TokenStream) -> TokenStream {
  class::methods_macro_impl(args, input)
}
