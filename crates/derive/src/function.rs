use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens};
use syn::spanned::Spanned;
use syn::{Expr, ItemFn, PatType, Receiver, Signature, Type, Visibility};

use crate::util::is_attr;

// TODO: don't fully clear attrs, only those we use

pub fn macro_impl(args: TokenStream, input: TokenStream) -> TokenStream {
  if !args.is_empty() {
    return syn::Error::new(Span::call_site(), "attribute does not accept any arguments")
      .into_compile_error()
      .into();
  }

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

  let input = syn::parse_macro_input!(input as syn::ItemFn);
  let vis = input.vis.clone();
  let sig = input.sig.clone();

  let fn_info = match FnInfo::parse(vis, &sig) {
    Ok(params) => params,
    Err(e) => return e.into_compile_error().into(),
  };

  if let Some(receiver) = &fn_info.receiver {
    return syn::Error::new(receiver.span(), "`self` is not supported")
      .into_compile_error()
      .into();
  }

  let mut input = input;
  clear_sig_attrs(&mut input.sig);

  emit_fn(
    &crate_name,
    fn_info.name.clone(),
    fn_info,
    Some(input),
    None,
  )
  .into()
}

pub fn emit_fn(
  crate_name: &Ident,
  out_fn_name: Ident,
  fn_info: FnInfo,
  input_fn: Option<ItemFn>,
  type_name: Option<Ident>,
) -> TokenStream2 {
  let input_fn_name = fn_info.name.clone();
  let vis = fn_info.vis.clone();
  let (input_mapping, arg_info) = emit_input_mapping(crate_name, &fn_info, type_name.as_ref());
  let args = arg_info.call_args;
  let this_arg = arg_info.this_input_arg;

  let call = if fn_info.receiver.is_some() {
    let type_name = type_name.as_ref();
    quote! {#type_name::#input_fn_name(this, #(#args),*)}
  } else {
    quote! {#input_fn_name(#(#args),*)}
  };

  quote! {
    #[allow(non_snake_case)]
    #vis fn #out_fn_name<'hebi>(
      ctx: &'hebi #crate_name::public::Context<'hebi>,
      #this_arg
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

      use #crate_name::util::check_args;
      use #crate_name::{FromHebi, FromHebiRef, IntoHebi};

      #input_fn

      #input_mapping

      #call.into_hebi(ctx)
    }
  }
}

pub fn emit_input_mapping(
  crate_name: &Ident,
  params: &FnInfo,
  type_name: Option<&Ident>,
) -> (TokenStream2, ArgInfo) {
  let from_hebi = format_ident!("from_hebi");
  let from_hebi_ref = format_ident!("from_hebi_ref");
  let args_ref = quote!(&args);
  let args_owned = quote!(args);
  let clone_call = Some(quote!(.clone()));
  let no_clone_call = None;

  let required_positional_params = params.required_positional().map(|v| v.name.to_string());
  let max_positional_params = params.max_positional();

  let keyword_params = params
    .keyword
    .iter()
    .map(|v| (v.name.to_string(), v.default.is_some()))
    .map(|(k, r)| quote! {(#k, #r)});

  let positional_param_mapping = params
    .positional
    .iter()
    .enumerate()
    .map(|(i, p)| {
      let name = format_ident!("_pos_{i}");
      let ty = &p.ty;
      let (from_fn, args, clone_call) = if !is_ref(ty) {
        (&from_hebi, &args_owned, &clone_call)
      } else {
        (&from_hebi_ref, &args_ref, &no_clone_call)
      };
      let init = match &p.default {
        Some(v) => quote! {
          if args.len() <= #i {
            #v
          } else {
            <#ty>::#from_fn(ctx, #args[#i]#clone_call)?
          }
        },
        None => quote! {
          <#ty>::#from_fn(ctx, #args[#i]#clone_call)?
        },
      };

      (quote! {let #name = #init;}, name)
    })
    .collect::<Vec<_>>();

  let keyword_param_mapping = params
    .keyword
    .iter()
    .enumerate()
    .map(|(i, p)| {
      let name = format_ident!("_kw_{i}");
      let key = p.name.to_string();
      let ty = &p.ty;
      let (from_fn, clone_call) = if !is_ref(ty) {
        (&from_hebi, &clone_call)
      } else {
        (&from_hebi_ref, &no_clone_call)
      };
      let init = match &p.default {
        Some(v) => quote! {
          if let Some(value) = kwargs.as_ref().and_then(|kw| kw.get(#key)) {
            <#ty>::#from_fn(ctx, value #clone_call)?
          } else {
            #v
          }
        },
        None => quote! {
          <#ty>::#from_fn(ctx, kwargs.as_ref().and_then(|kw| kw.get(#key)).unwrap() #clone_call)?
        },
      };

      (quote! {let #name = #init;}, name)
    })
    .collect::<Vec<_>>();

  let (this_arg, this_mapping) = match params.receiver.as_ref().map(|r| r.mutability) {
    Some(m) => {
      let is_mut = m.is_some();
      let this_arg = match is_mut {
        true => quote!(mut this: #crate_name::public::UserData<'hebi>,),
        false => quote!(mut this: #crate_name::public::UserData<'hebi>,),
      };
      let cast_error_msg = format!(
        "class is not an instance of {}",
        type_name.as_ref().unwrap()
      );
      let cast = match is_mut {
        true => format_ident!("cast"),
        false => format_ident!("cast_mut"),
      };
      let this_mapping = quote! {
        let this = match this.#cast::<#type_name>() {
          Some(this) => this,
          None => return Err(#crate_name::Error::runtime(#cast_error_msg))
        };
      };
      (Some(this_arg), Some(this_mapping))
    }
    _ => (None, None),
  };

  let input_mapping = positional_param_mapping
    .iter()
    .map(|(t, _)| t)
    .chain(keyword_param_mapping.iter().map(|(t, _)| t));

  let args = positional_param_mapping
    .iter()
    .map(|(_, i)| i)
    .chain(keyword_param_mapping.iter().map(|(_, i)| i))
    .collect::<Vec<_>>();

  let out_args = ArgInfo {
    this_input_arg: this_arg,
    call_args: args.iter().map(|&i| i.clone()).collect(),
  };

  (
    quote! {
      #this_mapping

      check_args(
        args,
        kwargs.as_ref(),
        /* pos_required */ &[#(#required_positional_params),*],
        /* pos_max */ #max_positional_params,
        /* kw */ &[#(#keyword_params),*],
      )?;

      #(#input_mapping)*
    },
    out_args,
  )
}

pub fn clear_sig_attrs(sig: &mut Signature) {
  for input in sig.inputs.iter_mut() {
    match input {
      syn::FnArg::Receiver(Receiver { attrs, .. }) | syn::FnArg::Typed(PatType { attrs, .. }) => {
        *attrs = attrs
          .iter()
          .cloned()
          .filter(|a| !is_attr(a, &["kw", "default"]))
          .collect()
      }
    }
  }
}

pub struct ArgInfo {
  pub this_input_arg: Option<TokenStream2>,
  pub call_args: Vec<Ident>,
}

// TODO: rest argv/kwargs

pub struct FnInfo {
  pub name: Ident,
  pub vis: Visibility,
  pub receiver: Option<Receiver>,
  pub positional: Vec<Param>,
  pub keyword: Vec<Param>,
}

pub struct Param {
  pub name: Ident,
  pub ty: Type,
  pub default: Option<Expr>,
}

impl FnInfo {
  pub fn required_positional(&self) -> impl Iterator<Item = &Param> {
    self.positional.iter().filter(|v| v.default.is_none())
  }

  pub fn max_positional(&self) -> usize {
    self.positional.len()
  }
}

impl FnInfo {
  pub fn parse(vis: Visibility, sig: &Signature) -> syn::Result<Self> {
    if !sig.generics.params.is_empty() {
      return Err(syn::Error::new(
        sig.generics.span(),
        "generics are not supported",
      ));
    }

    let name = sig.ident.clone();
    let inputs = &sig.inputs;
    let mut receiver = None;
    let mut positional = vec![];
    let mut keyword = vec![];

    enum State {
      Positional,
      PositionalDefault,
      Keyword,
    }

    let mut state = State::Positional;

    for param in inputs.iter() {
      match param {
        syn::FnArg::Receiver(r) => {
          if r.reference.is_none() {
            return Err(syn::Error::new(
              r.span(),
              "receiver must be taken by reference",
            ));
          }
          receiver = Some(r.clone());
        }
        syn::FnArg::Typed(r) => {
          let name = get_name(r)?;
          let ty = r.ty.as_ref().clone();
          let default = get_default(r)?;

          if !is_keyword(r) {
            if matches!(state, State::Keyword) {
              return Err(syn::Error::new(
                name.span(),
                "positional parameters may not appear after keyword parameters",
              ));
            }

            if default.is_some() {
              state = State::PositionalDefault;
            } else if matches!(state, State::PositionalDefault) {
              return Err(syn::Error::new(
                name.span(),
                "non-default positional parameters may not appear after default positional parameters",
              ));
            }
          } else {
            state = State::Keyword;
          }

          let param = Param { name, ty, default };
          if is_keyword(r) {
            keyword.push(param);
          } else {
            positional.push(param)
          }
        }
      }
    }

    Ok(FnInfo {
      name,
      vis,
      receiver,
      positional,
      keyword,
    })
  }
}

pub fn get_name(param: &PatType) -> syn::Result<Ident> {
  if let syn::Pat::Ident(pat) = &*param.pat {
    Ok(pat.ident.clone())
  } else {
    Err(syn::Error::new(
      param.pat.span(),
      "param pattern must be an identifier",
    ))
  }
}

pub fn is_keyword(param: &PatType) -> bool {
  param.attrs.iter().any(|v| v.path.is_ident("kw"))
}

pub fn is_option(param: &PatType) -> bool {
  match &*param.ty {
    Type::Path(ty) if ty.path.segments.len() == 1 => match ty.path.segments.first() {
      Some(segment) => segment.ident == "Option" && !segment.arguments.is_empty(),
      None => false,
    },
    _ => false,
  }
}

pub fn get_default(param: &PatType) -> syn::Result<Option<Expr>> {
  let default = param
    .attrs
    .iter()
    .find(|v| v.path.is_ident("default"))
    .map(|v| v.parse_args())
    .transpose()?;
  if default.is_none() && is_option(param) {
    Ok(Some(syn::parse_quote!(None)))
  } else {
    Ok(default)
  }
}

pub fn is_ref(ty: &Type) -> bool {
  match ty {
    Type::Path(ty) if ty.path.segments.len() == 1 => match ty.path.segments.first() {
      Some(syn::PathSegment {
        ident,
        arguments: syn::PathArguments::AngleBracketed(inner),
      }) => {
        ident == "Option"
          && inner
            .args
            .first()
            .and_then(|v| match v {
              syn::GenericArgument::Type(ty) => Some(ty),
              _ => None,
            })
            .map(is_direct_ref)
            .unwrap_or(false)
      }
      _ => false,
    },
    Type::Reference(_) => true,
    _ => false,
  }
}

pub fn is_direct_ref(ty: &Type) -> bool {
  matches!(ty, Type::Reference(_))
}
