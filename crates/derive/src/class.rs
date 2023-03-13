use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{Attribute, ImplItem, ImplItemMethod, ItemFn, ItemImpl, ItemStruct, Type, Visibility};

use crate::function;
use crate::function::{clear_sig_attrs, FnInfo};
use crate::util::is_attr;

// TODO: ensure generics are not allowed
// TODO: static methods?

pub fn class_macro_impl(args: TokenStream, input: TokenStream) -> TokenStream {
  let input = syn::parse_macro_input!(input as ItemStruct);

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

  let class = match Class::parse(&input) {
    Ok(class) => class,
    Err(e) => return e.into_compile_error().into(),
  };
  let (init_base, init_tag) = {
    let tag_ident = format_ident!("_{}__InitTag", class.name);
    let trait_ident = format_ident!("_{}__Init", class.name);

    (
      quote! {
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        struct #tag_ident {}
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        trait #trait_ident {
          fn init(self) -> Option<#crate_name::public::FunctionPtr>;
        }
        impl #trait_ident for &#tag_ident {
          fn init(self) -> Option<#crate_name::public::FunctionPtr> {
            None
          }
        }
      },
      tag_ident,
    )
  };
  let (methods_base, methods_tag) = {
    let tag_ident = format_ident!("_{}__MethodsTag", class.name);
    let trait_ident = format_ident!("_{}__Methods", class.name);

    (
      quote! {
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        struct #tag_ident {}
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        trait #trait_ident {
          fn methods(self) -> &'static [(&'static str, #crate_name::public::FunctionPtr)];
          fn static_methods(self) -> &'static [(&'static str, #crate_name::public::FunctionPtr)];
        }
        impl #trait_ident for &#tag_ident {
          fn methods(self) -> &'static [(&'static str, #crate_name::public::FunctionPtr)] {
            &[]
          }
          fn static_methods(self) -> &'static [(&'static str, #crate_name::public::FunctionPtr)] {
            &[]
          }
        }
      },
      tag_ident,
    )
  };
  let accessors = class
    .fields
    .iter()
    .map(|f| (f, f.emit_pair(&class.name, &crate_name)))
    .collect::<Vec<_>>();
  let accessor_fns = accessors.iter().map(|(_, (get, set))| {
    let get_fn = &get.0;
    let set_fn = set.as_ref().map(|v| &v.0);
    quote! {
      #get_fn
      #set_fn
    }
  });

  let type_info = {
    let class_name = class.name;
    let class_name_str = class_name.to_string();
    let none_expr = quote!(None);
    let class_fields = accessors.iter().map(|(field, (get, set))| {
      let name = field.name.to_string();
      let get_fn = &get.1;
      let set_fn = set
        .as_ref()
        .map(|(_, name)| quote!(Some(#name)))
        .unwrap_or_else(|| none_expr.clone());
      quote! {
        (#name, #get_fn, #set_fn)
      }
    });
    quote! {
      impl #crate_name::public::TypeInfo for #class_name {
        fn name() -> &'static str {
          #class_name_str
        }
        fn init() -> Option<#crate_name::public::FunctionPtr> {
          #init_tag {}.init()
        }
        fn fields() -> &'static [(&'static str, #crate_name::public::FunctionPtr, Option<#crate_name::public::FunctionPtr>)] {
          &[#(#class_fields),*]
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

  // TODO: clear fn arg attrs as well
  let mut input = input;
  for field in input.fields.iter_mut() {
    field.attrs = field
      .attrs
      .iter()
      .cloned()
      .filter(|a| !is_attr(a, &["get", "getset"]))
      .collect();
  }

  quote! {
    #input

    #init_base
    #methods_base
    #(#accessor_fns)*
    #type_info
  }
  .into()
}

struct Class {
  name: Ident,
  fields: Vec<Field>,
}

impl Class {
  fn parse(input: &ItemStruct) -> syn::Result<Self> {
    let name = input.ident.clone();
    let mut fields = vec![];
    let input_fields = match &input.fields {
      syn::Fields::Named(fields) => fields.named.iter(),
      syn::Fields::Unnamed(_) | syn::Fields::Unit => {
        return Err(syn::Error::new(
          input.span(),
          "only structs with named fields may be classes",
        ))
      }
    };
    for field in input_fields {
      if let Some(kind) = FieldKind::from_attrs(&field.attrs)? {
        fields.push(Field {
          name: field.ident.clone().unwrap(),
          ty: field.ty.clone(),
          kind,
        })
      }
    }

    Ok(Self { name, fields })
  }
}

struct Field {
  name: Ident,
  ty: Type,
  kind: FieldKind,
}

impl Field {
  fn emit_pair(
    &self,
    type_name: &Ident,
    crate_name: &Ident,
  ) -> ((TokenStream2, Ident), Option<(TokenStream2, Ident)>) {
    match self.kind {
      FieldKind::Get => (self.emit_getter(type_name, crate_name), None),
      FieldKind::GetSet => (
        self.emit_getter(type_name, crate_name),
        Some(self.emit_setter(type_name, crate_name)),
      ),
    }
  }

  fn emit_getter(&self, type_name: &Ident, crate_name: &Ident) -> (TokenStream2, Ident) {
    let field_name = &self.name;
    let getter_fn_ident = format_ident!("_{}__get__{}", type_name, field_name);
    let cast_error_msg = format!("class is not an instance of {}", type_name);
    // TODO: generate `this` mapping using a separate function, so it can be reused
    (
      quote! {
        #[doc(hidden)]
        #[allow(non_snake_case)]
        fn #getter_fn_ident <'hebi>(
          ctx: &'hebi #crate_name::public::Context<'hebi>,
          mut this: #crate_name::public::Value<'hebi>,
          args: &'hebi [#crate_name::public::Value<'hebi>],
          kwargs: Option<#crate_name::public::Dict<'hebi>>,
        ) -> #crate_name::Result<#crate_name::public::Value<'hebi>> {
          use #crate_name::IntoHebi;
          let this = match this.as_user_data() {
            Some(this) => this,
            None => return Err(#crate_name::Error::runtime("getter got receiver which is not user data")),
          };
          let this = match unsafe { this.cast::<#type_name>() } {
            Some(this) => this,
            None => return Err(#crate_name::Error::runtime(#cast_error_msg))
          };
          this.#field_name.into_hebi(ctx)
        }
      },
      getter_fn_ident,
    )
  }

  fn emit_setter(&self, type_name: &Ident, crate_name: &Ident) -> (TokenStream2, Ident) {
    let field_name = &self.name;
    let field_ty = &self.ty;
    let setter_fn_ident = format_ident!("_{}__set__{}", type_name, field_name);
    let cast_error_msg = format!("class is not an instance of {}", type_name);
    (
      quote! {
        #[doc(hidden)]
        #[allow(non_snake_case)]
        fn #setter_fn_ident <'hebi>(
          ctx: &'hebi #crate_name::public::Context<'hebi>,
          mut this: #crate_name::public::Value<'hebi>,
          args: &'hebi [#crate_name::public::Value<'hebi>],
          kwargs: Option<#crate_name::public::Dict<'hebi>>,
        ) -> #crate_name::Result<#crate_name::public::Value<'hebi>> {
          use #crate_name::{FromHebi, IntoHebi};
          let mut this = match this.as_user_data() {
            Some(this) => this,
            None => return Err(#crate_name::Error::runtime("setter got receiver which is not user data")),
          };
          let mut this = match unsafe { this.cast_mut::<#type_name>() } {
            Some(this) => this,
            None => return Err(#crate_name::Error::runtime(#cast_error_msg))
          };
          let value = match args.get(0).cloned() {
            Some(value) => value,
            None => return Err(#crate_name::Error::runtime("setter expects value as 2nd argument, got none")),
          };
          this.#field_name = <#field_ty>::from_hebi(ctx, value)?;
          ().into_hebi(ctx)
        }
      },
      setter_fn_ident,
    )
  }
}

enum FieldKind {
  Get,
  GetSet,
}

impl FieldKind {
  fn from_attrs(attrs: &[Attribute]) -> syn::Result<Option<Self>> {
    let mut out = None;
    for attr in attrs {
      if attr.path.leading_colon.is_none() && attr.path.segments.len() == 1 {
        let ident = &attr.path.segments.first().unwrap().ident;
        if ident == "get" {
          if out.is_some() {
            return Err(syn::Error::new(
              attr.span(),
              "fields may only have one `get`/`getset` annotation",
            ));
          }
          out = Some(FieldKind::Get);
        } else if ident == "getset" {
          if out.is_some() {
            return Err(syn::Error::new(
              attr.span(),
              "fields may only have one `get`/`getset` annotation",
            ));
          }
          out = Some(FieldKind::GetSet);
        }
      }
    }
    Ok(out)
  }
}

pub fn methods_macro_impl(args: TokenStream, input: TokenStream) -> TokenStream {
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

  let init_impl = if let Some(init) = &methods.init {
    let tag_ident = format_ident!("_{}__InitTag", methods.type_name);
    let trait_ident = format_ident!("_{}__Init", methods.type_name);
    let fn_ident = format_ident!("_{}__call__{}", methods.type_name, init.sig.ident);
    let type_name = &methods.type_name;

    let fn_info = match FnInfo::parse(Visibility::Inherited, &init.sig) {
      Ok(fn_info) => fn_info,
      Err(e) => return e.into_compile_error().into(),
    };

    let (input_mapping, arg_info) =
      function::emit_input_mapping(&crate_name, &fn_info, Some(&methods.type_name));
    let input_fn_name = init.sig.ident.clone();
    let args = &arg_info.call_args;
    let call = quote! {#input_fn_name(#(#args),*)};

    Some(quote! {
      #[doc(hidden)]
      #[allow(non_snake_case)]
      fn #fn_ident<'hebi>(
        ctx: &'hebi #crate_name::public::Context<'hebi>,
        mut this: #crate_name::public::Value<'hebi>,
        args: &'hebi [#crate_name::public::Value<'hebi>],
        kwargs: Option<#crate_name::public::Dict<'hebi>>,
      ) -> #crate_name::Result<#crate_name::public::Value<'hebi>> {
        #![allow(
          clippy::unnecessary_lazy_evaluations,
          clippy::absurd_extreme_comparisons,
          unused_imports,
          unused_variables,
          dead_code
        )]

        use #crate_name::{FromHebi, IntoHebi, public::IntoUserData};
        use #crate_name::util::check_args;

        #input_mapping
        #type_name::#call.into_user_data(ctx)
      }

      impl #trait_ident for #tag_ident {
        fn init(self) -> Option<#crate_name::public::FunctionPtr> {
          Some(#fn_ident)
        }
      }
    })
  } else {
    None
  };

  let methods_impl = {
    let tag_ident = format_ident!("_{}__MethodsTag", methods.type_name);
    let trait_ident = format_ident!("_{}__Methods", methods.type_name);
    let mut bindings = vec![];
    let mut method_list = vec![];
    let mut static_method_list = vec![];
    for method in methods.other.iter() {
      let out_fn_name = format_ident!("_{}__call__{}", methods.type_name, method.sig.ident);
      let fn_info = match FnInfo::parse(Visibility::Inherited, &method.sig) {
        Ok(fn_info) => fn_info,
        Err(e) => return e.into_compile_error().into(),
      };
      bindings.push(function::emit_fn(
        &crate_name,
        out_fn_name.clone(),
        fn_info,
        None,
        Some(methods.type_name.clone()),
        true,
      ));
      let sig_name = method.sig.ident.to_string();
      if method.sig.receiver().is_some() {
        method_list.push(quote!((#sig_name, #out_fn_name)));
      } else {
        static_method_list.push(quote!((#sig_name, #out_fn_name)));
      }
    }
    quote! {
      #(#bindings)*

      impl #trait_ident for #tag_ident {
        fn methods(self) -> &'static [(&'static str, #crate_name::public::FunctionPtr)] {
          &[#(#method_list),*]
        }
        fn static_methods(self) -> &'static [(&'static str, #crate_name::public::FunctionPtr)] {
          &[#(#static_method_list),*]
        }
      }
    }
  };

  let mut input = input;
  for item in input.items.iter_mut() {
    if let ImplItem::Method(m) = item {
      m.attrs = m
        .attrs
        .iter()
        .cloned()
        .filter(|a| !is_attr(a, &["init"]))
        .collect();

      clear_sig_attrs(&mut m.sig);
    }
  }

  quote! {
    #input

    #init_impl
    #methods_impl
  }
  .into()
}

struct Methods {
  type_name: Ident,
  init: Option<ImplItemMethod>,
  other: Vec<ImplItemMethod>,
}

impl Methods {
  fn parse(input: &ItemImpl) -> syn::Result<Self> {
    let type_name = *input.self_ty.clone();
    let type_name = match type_name {
      Type::Path(p)
        if p.qself.is_none() && p.path.leading_colon.is_none() && p.path.segments.len() == 1 =>
      {
        p.path.segments.first().unwrap().ident.clone()
      }
      _ => {
        return Err(syn::Error::new(
          input.self_ty.span(),
          "type must be an identifier with a single path segment",
        ))
      }
    };
    let mut init = None;
    let mut other = vec![];
    for func in input.items.iter().filter_map(|item| match item {
      ImplItem::Method(m) => Some(m),
      _ => None,
    }) {
      // TODO: init must not have receiver
      // TODO: methods must have &self or &mut self
      if matches!(func.vis, Visibility::Public(_)) {
        if func.attrs.iter().any(|a| is_attr(a, &["init"])) {
          if init.is_some() {
            return Err(syn::Error::new(
              func.span(),
              "only one method may be tagged with `#[init]`",
            ));
          }
          init = Some(func.clone());
        } else {
          other.push(func.clone());
        }
      }
    }

    Ok(Methods {
      type_name,
      init,
      other,
    })
  }
}
