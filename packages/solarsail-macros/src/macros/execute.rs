use crate::macros::utils::{generate_param_extractions, generate_struct_fields, snake_to_pascal};
use proc_macro2::TokenStream;
use quote::quote;
use std::ops::Deref;
use syn::{FnArg, ItemFn, PatType, Type, TypePath, Ident, Expr};

pub fn execute(func: &ItemFn, _args: Vec<(Ident, Expr)>) -> TokenStream {
  let func_name = &func.sig.ident;
  let func_vis = &func.vis;
  let func_block = &func.block;
  let func_attrs = &func.attrs;
  let func_result = &func.sig.output;

  // Process execute args
  // let mut authority: Option<String> = String::new();
  // for (key, value) in &args {
  //   match key.to_string().as_str() {
  //     "authority" => {
  //       authority = value.to_string();
  //     }
  //   }
  // }

  let args_struct_name = syn::Ident::new(
    &snake_to_pascal(&func_name.to_string()),
    proc_macro2::Span::call_site(),
  );

  // Extract function params, skip self & ctx
  let params: Vec<_> = func.sig.inputs.iter()
    .filter(|param| {
      match param {
        FnArg::Receiver(_) => false,
        FnArg::Typed(PatType { ty, .. }) => {
          // Skip param if type is ExecuteContext or &ExecuteContext
          match ty.deref() {
            Type::Path(TypePath { path, .. }) => {
              if let Some(segment) = path.segments.last() {
                return segment.ident != "ExecuteContext";
              }
            }
            Type::Reference(syn::TypeReference { elem, .. }) => {
              if let Type::Path(TypePath { path, .. }) = &**elem {
                if let Some(segment) = path.segments.last() {
                  return segment.ident != "ExecuteContext";
                }
              }
            }
            _ => {}
          }
          true
        }
      }
    })
    .cloned()
    .collect();

  // Generate struct fields and parameter extractions using utility functions
  let struct_fields = generate_struct_fields(&params);
  let param_extractions = generate_param_extractions(&params);

  // Create the execute function name
  let execute_func_name = syn::Ident::new(
    &format!("execute_{}", func_name),
    proc_macro2::Span::call_site(),
  );

  // Generate the args struct
  let args_struct = quote! {
    #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
    #func_vis struct #args_struct_name {
      #(#struct_fields)*
    }
  };

  // Generate the transformed function
  let transformed_func = quote! {
    #(#func_attrs)*
    #func_vis fn #execute_func_name(ctx: &mut ::solarsail::ExecuteContext, args: #args_struct_name) #func_result {
      // Extract parameters from args struct
      #(#param_extractions)*

      // Original function body
      #func_block
    }
  };

  quote! {
    #args_struct
    #transformed_func
  }
}