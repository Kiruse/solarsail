use crate::macros::utils::{extract_msg_params, generate_msg_extractions, extract_msg_meta, generate_struct_fields, MsgParams};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Ident, ItemFn};
use syn::spanned::Spanned;

pub fn transform(func: &ItemFn) -> Result<TokenStream, syn::Error> {
  if func.sig.ident != "migrate" {
    return Err(syn::Error::new(func.span(), "`migrate` transform can only be applied to a `migrate` function"));
  }

  let func_vis = &func.vis;
  let func_block = func.block.clone();
  let func_attrs = &func.attrs;
  let func_return = &func.sig.output;

  let msg = Ident::new("msg", Span::mixed_site());

  let params = extract_msg_params(&func.sig)?;

  let (msg_struct, param_extractions) = match params {
    MsgParams::Fields(fields) => {
      let struct_fields = generate_struct_fields(&fields);
      let param_extractions = generate_msg_extractions(&fields, msg.clone());

      let msg_struct = quote! {
        /// Contract-specific migrate message. You will most likely not need this unless you're
        /// writing a smart contract with only minor deviation from the original.
        #[::solarsail::solarize]
        pub struct MigrateMsg {
          #(#struct_fields)*
        }
      };

      (msg_struct, param_extractions)
    }
    MsgParams::Msg(arg) => {
      let (ty, param_extraction) = extract_msg_meta(msg.clone(), arg)?;
      (quote! { type MigrateMsg = #ty; }, param_extraction)
    }
  };

  let param_extractions = param_extractions.stmts;

  let transformed_func = quote! {
    #[cfg_attr(not(feature = "library"), ::solarsail::cw_std::entry_point)]
    #(#func_attrs)*
    #func_vis fn migrate(mut ctx: ::solarsail::ExecuteContext, #msg: MigrateMsg) #func_return {
      ::solarsail::cw2::set_contract_version(ctx.deps.storage, env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))?;

      #(#param_extractions)*

      let result: Result<(), _> = #func_block;

      match result {
        Ok(()) => {
          Ok(::solarsail::cw_std::Response::new().add_submessages(ctx.submsgs).add_events(ctx.events))
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
