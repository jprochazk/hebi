use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{ImplItem, ItemImpl};

use crate::class::{Methods, ReceiverMapping};

pub fn macro_impl(_: TokenStream, input: TokenStream) -> TokenStream {
  let input = syn::parse_macro_input!(input as ItemImpl);

  let crate_name = match proc_macro_crate::crate_name("hebi") {
    Ok(found) => match found {
      proc_macro_crate::FoundCrate::Itself => format_ident!("crate"),
      proc_macro_crate::FoundCrate::Name(name) => format_ident!("{name}"),
    },
    Err(e) => {
      return syn::Error::new(Span::call_site(), format!("{e}"))
        .into_compile_error()
        .into()
    }
  };

  let methods = match Methods::parse(&input) {
    Ok(methods) => methods,
    Err(e) => return e.into_compile_error().into(),
  };

  if let Some(init) = &methods.init {
    return syn::Error::new(init.span(), "builtins may not have initializers")
      .into_compile_error()
      .into();
  }

  let receiver_mapping = {
    let bad_receiver_err = format!("receiver is not an instance of `{}`", methods.type_name);
    ReceiverMapping {
      ref_: quote! {
        let mut args = args;
        let args = args.resolve_receiver()?;
        let this = match args.this().clone().unbind().to_object() {
          Some(this) => this,
          None => return Err(#crate_name::Error::runtime(#bad_receiver_err)),
        };
        let this = unsafe { this._get() };
      },
      mut_: quote! {
        let mut args = args;
        let args = args.resolve_receiver()?;
        let mut this = match args.this().clone().unbind().to_object() {
          Some(this) => this,
          None => return Err(#crate_name::Error::runtime(#bad_receiver_err)),
        };
        let mut this = unsafe { this._get_mut() };
      },
    }
  };

  let methods_impl =
    match methods.gen_methods_impl(&crate_name, Some(&receiver_mapping), Some("builtin_")) {
      Ok(v) => v,
      Err(e) => return e.into_compile_error().into(),
    };

  let type_info = {
    let class_name = &methods.type_name;
    let methods_tag = format_ident!("_{}__MethodsTag", class_name);
    let methods_trait = format_ident!("_{}__Methods", class_name);
    let class_name_str = class_name.to_string();
    quote! {
      #[doc(hidden)]
      #[allow(non_camel_case_types)]
      struct #methods_tag {}
      #[doc(hidden)]
      #[allow(non_camel_case_types)]
      trait #methods_trait {
        fn methods(self) -> &'static [(&'static str, #crate_name::public::FunctionPtr)];
        fn static_methods(self) -> &'static [(&'static str, #crate_name::public::FunctionPtr)];
      }
      impl #methods_trait for &#methods_tag {
        fn methods(self) -> &'static [(&'static str, #crate_name::public::FunctionPtr)] {
          &[]
        }
        fn static_methods(self) -> &'static [(&'static str, #crate_name::public::FunctionPtr)] {
          &[]
        }
      }
      impl #crate_name::public::TypeInfo for #class_name {
        fn name() -> &'static str {
          #class_name_str
        }
        fn init() -> Option<#crate_name::public::FunctionPtr> {
          None
        }
        fn fields() -> &'static [(&'static str, #crate_name::public::FunctionPtr, Option<#crate_name::public::FunctionPtr>)] {
          &[]
        }
        fn methods() -> &'static [(&'static str, #crate_name::public::FunctionPtr)] {
          #methods_tag {}.methods()
        }
        fn static_methods() -> &'static [(&'static str, #crate_name::public::FunctionPtr)] {
          #methods_tag {}.static_methods()
        }
      }
    }
  };

  let mut input = input;
  for item in input.items.iter_mut() {
    if let ImplItem::Method(m) = item {
      m.sig.ident = format_ident!("builtin_{}", m.sig.ident);
    }
  }

  quote! {
    #input

    #methods_impl
    #type_info
  }
  .into()
}
