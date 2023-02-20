use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{FnArg, ImplItemMethod, ItemImpl, Visibility};

pub fn macro_impl(args: TokenStream, input: TokenStream) -> TokenStream {
  if !args.is_empty() {
    return syn::Error::new(Span::call_site(), "attribute does not accept any arguments")
      .into_compile_error()
      .into();
  }

  let input = syn::parse_macro_input!(input as ItemImpl);

  let generics = input.generics.clone();
  let self_ty = input.self_ty.clone();
  let mut methods = vec![];
  for item in input.items.iter() {
    match item {
      syn::ImplItem::Method(method) => {
        if matches!(
          method.vis,
          Visibility::Public(_) | Visibility::Crate(_) | Visibility::Restricted(_)
        ) && matches!(method.sig.inputs.first(), Some(FnArg::Receiver(..)))
        {
          let ImplItemMethod {
            attrs,
            vis,
            defaultness,
            sig,
            ..
          } = method;
          let FnArg::Receiver(receiver) = sig.inputs.first().unwrap() else { unreachable!() };
          let getter = if receiver.mutability.is_some() {
            Ident::new("_get_mut", Span::call_site())
          } else {
            Ident::new("_get", Span::call_site())
          };

          let name = sig.ident.clone();
          let args = sig.inputs.iter().filter_map(|input| match input {
            FnArg::Receiver(_) => None,
            FnArg::Typed(v) => match &*v.pat {
              syn::Pat::Ident(v) => Some(v.ident.clone()),
              _ => None,
            },
          });
          methods.push(quote! {
            #(#attrs)*
            #vis #defaultness #sig {
              unsafe { self.#getter() }.#name(#(#args),*)
            }
          });
        }
      }
      _ => todo!(),
    }
  }

  quote! {
    #input

    impl #generics crate::value::object::Handle<#self_ty> {
      #(#methods)*
    }
  }
  .into()
}
