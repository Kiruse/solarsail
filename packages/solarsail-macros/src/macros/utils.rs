use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, Pat, PatType};

/// Convert snake_case string to PascalCase
pub fn snake_to_pascal(input: &str) -> String {
  input.split('_')
    .map(|word| {
      let mut chars = word.chars();
      match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
      }
    })
    .collect::<Vec<_>>()
    .join("")
}

/// Generate struct fields from function parameters
pub fn generate_struct_fields(params: &[FnArg]) -> Vec<TokenStream> {
  params.iter().map(|param| {
    match param {
      FnArg::Receiver(_) => quote! {},
      FnArg::Typed(PatType { pat, ty, .. }) => {
        if let Pat::Ident(pat_ident) = &**pat {
          let field_name = &pat_ident.ident;
          let field_type = &**ty;
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
pub fn generate_param_extractions(params: &[FnArg]) -> Vec<TokenStream> {
  params.iter().map(|param| {
    match param {
      FnArg::Typed(PatType { pat, ty: _ty, .. }) => {
        if let Pat::Ident(pat_ident) = &**pat {
          let param_name = &pat_ident.ident;
          quote! {
            let #param_name = args.#param_name;
          }
        } else {
          quote! {}
        }
      }
      FnArg::Receiver(_) => quote! {},
    }
  }).collect()
}
