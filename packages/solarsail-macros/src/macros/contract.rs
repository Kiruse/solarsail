use proc_macro2::TokenStream;
use syn::{ItemMod, FnArg, PatType, spanned::Spanned, Type, TypePath, Pat};
use quote::quote;

pub fn contract(input: &ItemMod) -> TokenStream {
  let mod_contents = &input.content;

  if mod_contents.is_none() {
    return syn::Error::new(
      input.span(),
      "Module must contain at least one item"
    ).to_compile_error().into();
  }

  let (_, items) = mod_contents.as_ref().unwrap();

  // Process items to find functions with authority attributes
  let mut generated_code = Vec::new();

  for item in items {
    match item {
      syn::Item::Fn(func) => {
        // Check if function has authority attribute
        let has_authority = func.attrs.iter().any(|attr| {
          attr.path().is_ident("authority")
        });

        if has_authority {
          // Generate code for authority function
          let func_name = &func.sig.ident;
          let func_vis = &func.vis;
          let func_block = &func.block;

          // Extract parameters for authority checking
          let authority_params: Vec<_> = func.sig.inputs.iter()
            .filter_map(|arg| {
              if let FnArg::Typed(PatType { pat, ty, .. }) = arg {
                if let Pat::Ident(pat_ident) = &**pat {
                  Some((pat_ident.ident.clone(), (**ty).clone()))
                } else {
                  None
                }
              } else {
                None
              }
            })
            .collect();

          // Generate authority checking code
          let authority_checks: Vec<_> = authority_params.iter()
            .map(|(param_name, param_type)| {
              if let Type::Path(TypePath { path, .. }) = param_type {
                if let Some(segment) = path.segments.last() {
                  if segment.ident == "String" || segment.ident == "Addr" {
                    quote! {
                      // Authority check for parameter: #param_name
                      if #param_name.is_empty() {
                        return Err(cosmwasm_std::StdError::generic_err("Authority required"));
                      }
                    }
                  } else {
                    quote! {}
                  }
                } else {
                  quote! {}
                }
              } else {
                quote! {}
              }
            })
            .collect();

          let generated_func = quote! {
            #func_vis fn #func_name(mut deps: cosmwasm_std::DepsMut, env: cosmwasm_std::Env, info: cosmwasm_std::MessageInfo, msg: #func_name) -> Result<cosmwasm_std::Response, cosmwasm_std::ContractError> {
              #(#authority_checks)*

              // Original function logic would go here
              #func_block
            }
          };

          generated_code.push(generated_func);
        }
      }
      _ => {}
    }
  }

  // Return the original module plus generated code
  let expanded = quote! {
    #input

    // Generated authority functions
    #(#generated_code)*
  };

  TokenStream::from(expanded)
}
