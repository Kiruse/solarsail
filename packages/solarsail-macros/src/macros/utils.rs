use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Block, FnArg, Ident, Pat, PatType, Path, Signature, Type, TypePath, parse_quote};
use syn::spanned::Spanned;

pub enum MsgParams<'a> {
  Fields(Vec<&'a FnArg>),
  Msg(&'a FnArg),
}

pub fn extract_msg_params<'a>(sig: &'a Signature) -> Result<MsgParams<'a>, syn::Error> {
  let mut params = sig.inputs
    .iter()
    // skip self
    .filter(|param| {
      match param {
        FnArg::Receiver(_) => false,
        _ => true,
      }
    })
    .collect::<Vec<_>>();

  // skip the first parameter if it's a QueryContext or ExecuteContext
  if is_ctx(params.first().unwrap()) {
    params.remove(0);
  }

  if let Some(param) = params.iter().find(|p| is_ctx(p)) {
    return Err(syn::Error::new(param.span(), "QueryContext or ExecuteContext must be the first parameter"));
  }

  let msg_params = params
    .iter()
    .filter(|p| {
      if let FnArg::Typed(PatType { attrs, .. }) = p {
        attrs.iter().any(|attr| attr.path().is_ident("msg"))
      } else {
        false
      }
    })
    .collect::<Vec<_>>();
  if msg_params.len() > 1 {
    return Err(syn::Error::new(msg_params[1].span(), "Only one #[msg] attribute is allowed"));
  }
  if !msg_params.is_empty() && msg_params.len() != params.len() {
    return Err(syn::Error::new(params[msg_params.len()].span(), "Cannot mix #[msg] with regular parameters"));
  }

  if msg_params.is_empty() {
    Ok(MsgParams::Fields(params))
  } else {
    Ok(MsgParams::Msg(msg_params[0]))
  }
}

fn is_ctx(param: &FnArg) -> bool {
  if let FnArg::Typed(PatType { pat, .. }) = param {
    match &**pat {
      Pat::Ident(pat_ident) if pat_ident.ident == "ctx" || pat_ident.ident == "_ctx" => true,
      _ => false,
    }
  } else {
    false
  }
}

/// Generate struct fields from function parameters
pub fn generate_struct_fields(params: &[&FnArg]) -> Vec<TokenStream> {
  params.iter().map(|param| {
    match param {
      FnArg::Receiver(_) => quote! {},
      FnArg::Typed(PatType { pat, ty, .. }) => {
        if let Pat::Ident(pat_ident) = &**pat {
          let field_name = msg_field_name(&pat_ident.ident);
          let ty_name = format!("{}", quote! { #ty }.to_string());

          // Convert Addr to String in message structs
          let opt_addr = [
            "Option < Addr >",
            "Option < cosmwasm_std :: Addr >",
            "Option < :: cosmwasm_std :: Addr >",
          ].contains(&ty_name.as_str());
          let field_type = if ty_name == "Addr" {
            quote! { String }
          } else if opt_addr {
            quote! { Option<String> }
          } else {
            quote! { #ty }
          };

          quote! {
            pub #field_name: #field_type,
          }
        } else {
          quote! {}
        }
      }
    }
  }).collect()
}

/// Generate parameter extraction statements from args struct
pub fn generate_msg_extractions(params: &[&FnArg], args: Ident) -> Block {
  let stmts = params
    .iter()
    .map(|param| {
      match param {
        FnArg::Typed(PatType { pat, ty, .. }) => {
          if let Pat::Ident(pat_ident) = &**pat {
            let param_name = &pat_ident.ident;
            let field_name = msg_field_name(&pat_ident.ident);
            let ty_name = format!("{}", quote! { #ty }.to_string());

            // Check if the type is Addr or Option<Addr>
            let opt_addr = [
              "Option < Addr >",
              "Option < cosmwasm_std :: Addr >",
              "Option < :: cosmwasm_std :: Addr >",
            ].contains(&ty_name.as_str());
            if ty_name == "Addr" {
              quote! {
                let #param_name = ctx.deps.api.addr_validate(&#args.#field_name)?;
              }
            } else if opt_addr {
              quote! {
                let #param_name = #args.#field_name.as_ref().map(|addr| ctx.deps.api.addr_validate(addr)).transpose()?;
              }
            } else {
              quote! {
                let #param_name = #args.#field_name;
              }
            }
          } else {
            quote! {}
          }
        }
        FnArg::Receiver(_) => quote! {},
      }
    })
    .collect::<Vec<_>>();
  parse_quote! {{
    #(#stmts)*
  }}
}

pub fn extract_msg_meta(msg: Ident, arg: &FnArg) -> Result<(Path, Block), syn::Error> {
  match arg {
    FnArg::Receiver(_) => return Err(syn::Error::new(arg.span(), "Unexpected receiver parameter")),
    FnArg::Typed(PatType { pat, ty, .. }) if matches!(**pat, Pat::Ident(_)) => {
      if let Pat::Ident(pat_ident) = &**pat {
        let field_name = &pat_ident.ident;
        let path = match &**ty {
          Type::Path(TypePath { path, .. }) => path.clone(),
          _ => return Err(syn::Error::new(ty.span(), "Unsupported parameter type")),
        };
        Ok((path, parse_quote! {{ let #field_name = #msg; }}))
      } else {
        unreachable!()
      }
    }
    _ => return Err(syn::Error::new(arg.span(), "Unsupported parameter pattern")),
  }
}

fn msg_field_name(ident: &Ident) -> Ident {
  if ident.to_string().starts_with('_') {
    Ident::new(&ident.to_string()[1..], ident.span())
  } else {
    ident.clone()
  }
}

pub fn make_ident(name: impl AsRef<str>) -> Ident {
  Ident::new(name.as_ref(), proc_macro2::Span::mixed_site())
}

pub fn find_attr<'a>(attrs: &'a [Attribute], name: impl AsRef<str>) -> Option<&'a Attribute> {
  attrs.iter().find(|attr| attr.path().is_ident(name.as_ref()))
}

/// Rename an identifier with a format string & recasing.
/// The first substitute passed in is considered the base name which should have a `span()` method.
macro_rules! rename_ident {
  ($case:expr, $fmt:expr, $name:expr $(, $sub:expr),*) => {
    syn::Ident::new(&format!($fmt, $name, $($sub),*).to_case($case), $name.span())
  };
}
pub(crate) use rename_ident;
