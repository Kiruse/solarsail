use crate::macros::utils::{extract_msg_params, generate_msg_extractions, extract_msg_meta, generate_struct_fields, MsgParams};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Attribute, Ident, ItemFn};
use syn::spanned::Spanned;

pub fn transform(span: Span, func: &ItemFn) -> Result<TokenStream, syn::Error> {
  if func.sig.ident != "migrate" {
    return Err(syn::Error::new(func.span(), "`migrate` transform can only be applied to a `migrate` function"));
  }

  let func_vis = &func.vis;
  let func_block = func.block.clone();
  let func_attrs = func.attrs
    .iter()
    .filter(|attr| filter_attrs(attr))
    .collect::<Vec<_>>();
  let func_return = &func.sig.output;

  let msg = Ident::new("msg", span);

  let params = extract_msg_params(&func)?;

  let (msg_struct, param_extractions) = match params {
    MsgParams::Fields(fields) => {
      let struct_fields = generate_struct_fields(&fields);
      let param_extractions = generate_msg_extractions(&fields, msg.clone());

      let msg_struct = quote! {
        #[::cosmwasm_schema::cw_serde]
        pub struct MigrateMsg {
          #(#struct_fields)*
        }
      };

      (msg_struct, param_extractions)
    }
    MsgParams::Msg(arg) => {
      let (ty, param_extraction) = extract_msg_meta(msg.clone(), arg)?;
      (quote! { type MigrateMsg = #ty; }, vec![param_extraction])
    }
  };

  let transformed_func = quote! {
    #[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
    #(#func_attrs)*
    #func_vis fn migrate(ctx: ::solarsail::ExecuteContext, #msg: MigrateMsg) #func_return {
      let mut __solarsail_submsgs: Vec<cosmwasm_std::SubMsg> = Vec::new();

      #(#param_extractions)*

      let result: Result<(), _> = #func_block;

      match result {
        Ok(()) => {
          Ok(cosmwasm_std::Response::new().add_submessages(__solarsail_submsgs))
        }
        Err(e) => Err(e),
      }
    }
  };

  Ok(quote! {
    #msg_struct
    #transformed_func
  })
}

fn filter_attrs(attr: &Attribute) -> bool {
  !attr.path().is_ident("migrate") && attr.path().segments.first().unwrap().ident != "authority"
}
