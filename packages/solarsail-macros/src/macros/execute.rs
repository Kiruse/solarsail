use crate::macros::utils::{extract_msg_params, find_attr, generate_msg_extractions, extract_msg_meta, generate_struct_fields, MsgParams};
use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Attribute, Ident, ItemFn};
use syn::spanned::Spanned;

pub fn transform(span: Span, func: &ItemFn) -> Result<TokenStream, syn::Error> {
  let func_name = &func.sig.ident;
  let func_vis = &func.vis;
  let func_block = func.block.clone();
  let func_attrs = func.attrs
    .iter()
    .filter(|attr| filter_attrs(attr))
    .collect::<Vec<_>>();
  let func_return = &func.sig.output;

  let authority_check = match find_attr(&func.attrs, "authority") {
    Some(attr) => {
      let auth = attr.parse_args::<Ident>()
        .map_err(|e| syn::Error::new(attr.span(), format!("Invalid authority attribute: {}", e)))?;
      let variant = Ident::new(
        &auth.to_string().to_case(Case::Pascal),
        auth.span(),
      );
      Some(quote! { Authority::#variant.check(&ctx)?; })
    }
    None => None,
  };

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

  let execute_func_name = Ident::new(
    &format!("execute_{}", func_name),
    func.sig.ident.span(),
  );

  let transformed_func = quote! {
    #(#func_attrs)*
    #func_vis fn #execute_func_name(mut ctx: ::solarsail::ExecuteContext, #msg: #msg_struct_name) #func_return {
      #authority_check

      #(#param_extractions)*

      let result: ::std::result::Result<(), _> = #func_block;

      match result {
        Ok(()) => {
          Ok(::cosmwasm_std::Response::new().add_submessages(ctx.submsgs).add_events(ctx.events))
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

pub fn generate_entrypoint(fns: &[Ident], has_authority: bool) -> TokenStream {
  if fns.is_empty() {
    return quote! {};
  }

  let mut variants = fns
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
      quote! { #variant(#msg_name) }
    })
    .collect::<Vec<_>>();
  if has_authority {
    variants.push(quote! { TransferAuthority(AuthorityTransfer) });
  }

  let mut match_arms = fns
    .iter()
    .map(|func_name| {
      let variant = Ident::new(
        &func_name.to_string().to_case(Case::Pascal),
        func_name.span(),
      );
      let execute_func_name = Ident::new(&format!("execute_{}", func_name), func_name.span());
      quote! {
        ExecuteMsg::#variant(msg) => {
          #execute_func_name(ctx, msg).map_err(|e| ::cosmwasm_std::StdError::msg(e.to_string()))
        }
      }
    })
    .collect::<Vec<_>>();
  if has_authority {
    match_arms.push(quote! {
      ExecuteMsg::TransferAuthority(msg) => {
        msg.transfer(ctx).map_err(|e| ::cosmwasm_std::StdError::msg(e.to_string()))
      }
    });
  }

  quote! {
    #[::cosmwasm_schema::cw_serde]
    pub enum ExecuteMsg {
      #(#variants,)*
    }

    #[cfg_attr(not(feature = "library"), ::cosmwasm_std::entry_point)]
    pub fn execute(
      deps: ::cosmwasm_std::DepsMut,
      env: ::cosmwasm_std::Env,
      info: ::cosmwasm_std::MessageInfo,
      msg: ExecuteMsg,
    ) -> ::std::result::Result<::cosmwasm_std::Response, ::cosmwasm_std::StdError> {
      let mut ctx = ::solarsail::ExecuteContext::new(deps, env, info);
      match msg {
        #(#match_arms)*
      }
    }
  }
}

fn filter_attrs(attr: &Attribute) -> bool {
  !attr.path().is_ident("execute") && attr.path().segments.first().unwrap().ident != "authority"
}
