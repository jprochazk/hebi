use proc_macro::TokenStream;

mod delegate;

#[doc(hidden)]
#[proc_macro_attribute]
pub fn delegate_to_handle(args: TokenStream, input: TokenStream) -> TokenStream {
  delegate::macro_impl(args, input)
}
