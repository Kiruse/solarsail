use crate::macros::utils::{extract_msg_params, extract_msg_meta, generate_msg_extractions, generate_struct_fields, MsgParams};
use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Attribute, Ident, ItemFn};

pub fn transform(span: Span, func: &ItemFn) -> Result<TokenStream, syn::Error> {
  let func_name = &func.sig.ident;
  let func_vis = &func.vis;
  let func_block = func.block.clone();
  let func_attrs = func.attrs
    .iter()
    .filter(|attr| filter_attrs(attr))
    .collect::<Vec<_>>();
  let func_return = &func.sig.output;

  let msg = Ident::new("msg", span);

  let params = extract_msg_params(&func)?;

  let msg_struct_name = Ident::new(
    &format!("{}Msg", func_name.to_string().to_case(Case::Pascal)),
    func.sig.ident.span(),
  );

  let (msg_struct, param_extractions) = match params {
    MsgParams::Fields(fields) => {
      let struct_fields = generate_struct_fields(&fields);
      let param_extractions = generate_msg_extractions(&fields, msg.clone());

      let msg_struct = quote! {
        #[::cosmwasm_schema::cw_serde]
        pub struct #msg_struct_name {
          #(#struct_fields)*
        }
      };

      (msg_struct, param_extractions)
    }
    MsgParams::Msg(arg) => {
      let (ty, param_extraction) = extract_msg_meta(msg.clone(), arg)?;
      (quote! { type #msg_struct_name = #ty; }, vec![param_extraction])
    }
  };

  let query_func_name = query_name(func_name);

  let transformed_func = quote! {
    #(#func_attrs)*
    #func_vis fn #query_func_name(ctx: ::solarsail::QueryContext, #msg: #msg_struct_name) #func_return {
      #(#param_extractions)*
      #func_block
    }
  };

  Ok(quote! {
    #msg_struct
    #transformed_func
  })
}

/// Generate the main query entrypoint function and QueryMsg enum
pub fn generate_entrypoint(query_functions: &[Ident]) -> TokenStream {
  if query_functions.is_empty() {
    return quote! {};
  }

  let query_variants: Vec<_> = query_functions
    .iter()
    .map(|func_name| {
      let variant = Ident::new(
        &func_name.to_string().to_case(Case::Pascal),
        func_name.span(),
      );
      let msg_name = Ident::new(
        &format!("{}Msg", func_name.to_string().to_case(Case::Pascal)),
        func_name.span(),
      );
      quote! {
        #variant(#msg_name),
      }
    })
    .collect();

  let match_arms: Vec<_> = query_functions.iter().map(|func_name| {
    let fn_name = query_name(func_name);
    let variant = Ident::new(
      &func_name.to_string().to_case(Case::Pascal),
      func_name.span(),
    );
    quote! {
      QueryMsg::#variant(msg) => {
        let result = #fn_name(ctx, msg)
          .map_err(|e| ::cosmwasm_std::StdError::msg(e.to_string()))?;
        ::cosmwasm_std::to_json_binary(&result)
      }
    }
  }).collect();

  quote! {
    #[::cosmwasm_schema::cw_serde]
    pub enum QueryMsg {
      #(#query_variants)*
    }

    #[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
    pub fn query(
      deps: ::cosmwasm_std::Deps,
      env: ::cosmwasm_std::Env,
      msg: QueryMsg,
    ) -> ::std::result::Result<::cosmwasm_std::Binary, ::cosmwasm_std::StdError> {
      let ctx = ::solarsail::QueryContext::new(deps, env);
      match msg {
        #(#match_arms)*
      }
    }
  }
}

fn query_name(func_name: &Ident) -> Ident {
  Ident::new(
    &if !func_name.to_string().starts_with("query_") {
      format!("query_{}", func_name)
    } else {
      func_name.to_string()
    },
    func_name.span(),
  )
}

fn filter_attrs(attr: &Attribute) -> bool {
  !attr.path().is_ident("query")
}
