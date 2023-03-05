use proc_macro::TokenStream;

mod delegate;
mod function;

#[doc(hidden)]
#[proc_macro_attribute]
pub fn delegate_to_handle(args: TokenStream, input: TokenStream) -> TokenStream {
  delegate::macro_impl(args, input)
}

#[proc_macro_attribute]
pub fn function(args: TokenStream, input: TokenStream) -> TokenStream {
  function::macro_impl(args, input)
}
