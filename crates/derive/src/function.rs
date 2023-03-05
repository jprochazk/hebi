use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{format_ident, quote};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Expr, FnArg, PatType, Token, Type};

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

  let mut input = syn::parse_macro_input!(input as syn::ItemFn);
  let vis = input.vis.clone();
  let name = input.sig.ident.clone();
  let inputs = input.sig.inputs.clone();

  for arg in input.sig.inputs.iter_mut() {
    match arg {
      syn::FnArg::Receiver(_) => {}
      syn::FnArg::Typed(arg) => {
        arg.attrs.clear();
      }
    }
  }

  let params = match SigInfo::get_from_sig_inputs(&inputs) {
    Ok(params) => params,
    Err(e) => return e.into_compile_error().into(),
  };

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
          if let Some(value) = kwargs.get(#key) {
            <#ty>::#from_fn(ctx, value #clone_call)?
          } else {
            #v
          }
        },
        None => quote! {
          <#ty>::#from_fn(ctx, kwargs.get(#key).unwrap() #clone_call)?
        },
      };

      (quote! {let #name = #init;}, name)
    })
    .collect::<Vec<_>>();

  let input_mapping = positional_param_mapping
    .iter()
    .map(|(t, _)| t)
    .chain(keyword_param_mapping.iter().map(|(t, _)| t));

  let args = positional_param_mapping
    .iter()
    .map(|(_, i)| i)
    .chain(keyword_param_mapping.iter().map(|(_, i)| i));

  let kwargs = if !keyword_param_mapping.is_empty() {
    Some(quote!(let kwargs = kwargs.unwrap();))
  } else {
    None
  };

  let call = quote! {#name(#(#args),*)};

  quote! {
    #vis fn #name<'hebi>(
      ctx: &'hebi #crate_name::public::Context<'hebi>,
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

      #input

      check_args(
        args,
        kwargs.as_ref(),
        /* pos_required */ &[#(#required_positional_params),*],
        /* pos_max */ #max_positional_params,
        /* kw */ &[#(#keyword_params),*],
      )?;

      #kwargs
      #(#input_mapping)*

      #call.into_hebi(ctx)
    }
  }
  .into()
}

// TODO: rest argv/kwargs

struct SigInfo {
  positional: Vec<Param>,
  keyword: Vec<Param>,
}

struct Param {
  name: Ident,
  ty: Type,
  default: Option<Expr>,
}

impl SigInfo {
  fn required_positional(&self) -> impl Iterator<Item = &Param> {
    self.positional.iter().filter(|v| v.default.is_none())
  }

  fn max_positional(&self) -> usize {
    self.positional.len()
  }
}

impl SigInfo {
  fn get_from_sig_inputs(inputs: &Punctuated<FnArg, Token![,]>) -> syn::Result<Self> {
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
          return Err(syn::Error::new(
            r.self_token.span,
            "`self` is not supported",
          ))
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

    Ok(SigInfo {
      positional,
      keyword,
    })
  }
}

fn get_name(param: &PatType) -> syn::Result<Ident> {
  if let syn::Pat::Ident(pat) = &*param.pat {
    Ok(pat.ident.clone())
  } else {
    Err(syn::Error::new(
      param.pat.span(),
      "param pattern must be an identifier",
    ))
  }
}

fn is_keyword(param: &PatType) -> bool {
  param.attrs.iter().any(|v| v.path.is_ident("kw"))
}

fn is_option(param: &PatType) -> bool {
  match &*param.ty {
    Type::Path(ty) if ty.path.segments.len() == 1 => match ty.path.segments.first() {
      Some(segment) => segment.ident == "Option" && !segment.arguments.is_empty(),
      None => false,
    },
    _ => false,
  }
}

fn get_default(param: &PatType) -> syn::Result<Option<Expr>> {
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

fn is_ref(ty: &Type) -> bool {
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

fn is_direct_ref(ty: &Type) -> bool {
  matches!(ty, Type::Reference(_))
}