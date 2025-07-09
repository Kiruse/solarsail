use proc_macro2::{Span, TokenStream};
use syn::{spanned::Spanned, Ident, Item, ItemMod, Macro};
use quote::{quote, ToTokens};

use crate::macros::utils::has_attr;

#[derive(Debug, Default, PartialEq, Eq, Hash)]
pub enum ContractMode {
  #[default]
  Invalid,
  Full,
  Execute,
  Query,
}

impl From<Ident> for ContractMode {
  fn from(ident: Ident) -> Self {
    match ident.to_string().as_str() {
      "full" => ContractMode::Full,
      "execute" => ContractMode::Execute,
      "query" => ContractMode::Query,
      _ => ContractMode::Invalid,
    }
  }
}

impl ContractMode {
  pub fn allows_execute(&self) -> bool {
    self == &ContractMode::Full || self == &ContractMode::Execute
  }

  pub fn allows_query(&self) -> bool {
    self == &ContractMode::Full || self == &ContractMode::Query
  }
}

pub fn contract(mode: ContractMode, input: &ItemMod) -> TokenStream {
  match contract_impl(mode, input) {
    Ok(expanded) => expanded,
    Err(e) => e.to_compile_error().into(),
  }
}

fn contract_impl(mode: ContractMode, input: &ItemMod) -> Result<TokenStream, syn::Error> {
  let span = Span::mixed_site();
  let mut input = input.clone();

  if input.content.is_none() {
    return Err(syn::Error::new(
      input.span(),
      "Module must contain at least one item"
    ));
  }

  let (_, items) = input.content.as_mut().unwrap();

  let authority_code = gen_authority(items)?;
  let mut has_instantiate = false;
  let mut has_migrate = false;
  let mut executors = Vec::new();
  let mut queriers = Vec::new();
  let mut errors = Vec::new();

  let mut i = 0usize;
  while i < items.len() {
    let sub: Option<TokenStream> = match &items[i] {
      Item::Fn(func) if func.sig.ident == "instantiate" => {
        has_instantiate = true;
        Some(crate::macros::instantiate::transform(span, func)?)
      }
      Item::Fn(func) if func.sig.ident == "migrate" => {
        has_migrate = true;
        Some(crate::macros::migrate::transform(span, func)?)
      }
      Item::Fn(func) if has_attr(&func.attrs, "execute") => {
        if !mode.allows_execute() {
          return Err(syn::Error::new(func.span(), "Execute function not allowed in this contract mode"));
        }
        executors.push(func.sig.ident.clone());
        Some(crate::macros::execute::transform(span, func)?)
      }
      Item::Fn(func) if has_attr(&func.attrs, "query") => {
        if !mode.allows_query() {
          return Err(syn::Error::new(func.span(), "Query function not allowed in this contract mode"));
        }
        queriers.push(func.sig.ident.clone());
        Some(crate::macros::query::transform(span, func)?)
      }
      Item::Macro(mac) if mac.mac.path.is_ident("error") => {
        let err = syn::parse2::<crate::parsers::ErrorDef>(mac.mac.tokens.clone())?;
        errors.push(err);
        Some(quote! {})
      }
      _ => None,
    };

    if let Some(ts) = sub {
      let mut file = syn::parse2::<syn::File>(ts)?;
      let count = file.items.len();
      items.splice(i..=i, file.items.drain(..));
      i += count;
    } else {
      i += 1;
    }
  }

  // instantiate function assertions
  if mode == ContractMode::Full && !has_instantiate {
    return Err(syn::Error::new(span, "Contract is missing an `instantiate` function"));
  } else if mode != ContractMode::Full && has_instantiate {
    return Err(syn::Error::new(span, "`instantiate` function must be located in the contract root"));
  }

  // migrate function assertions (optional)
  if mode != ContractMode::Full && has_migrate {
    return Err(syn::Error::new(span, "`migrate` function must be located in the contract root"));
  }

  let error_struct = if mode == ContractMode::Full {
    Some(crate::macros::error::generate_error_struct(&errors))
  } else if !errors.is_empty() {
    return Err(syn::Error::new(span, "`error` macro must be located in the contract root"));
  } else {
    None
  };

  let execute_entrypoint = crate::macros::execute::generate_entrypoint(&executors, authority_code.is_some());
  let query_entrypoint = crate::macros::query::generate_entrypoint(&queriers);

  items.extend(syn::parse2::<syn::File>(quote! {
    #error_struct
    #authority_code
    #execute_entrypoint
    #query_entrypoint
  })?.items);

  Ok(input.to_token_stream())
}

/// Handle authority logic for state! macro calls
fn gen_authority(items: &mut Vec<Item>) -> Result<Option<TokenStream>, syn::Error> {
  let mut authority_info = None;

  // find `state!` macro & extract authority fields
  for item in items.iter() {
    if let Item::Macro(macro_item) = item {
      let Macro { path, tokens, .. } = &macro_item.mac;
      if let Some(segment) = path.segments.last() {
        if segment.ident == "state" {
          let parsed = syn::parse2::<crate::macros::state::StateMacroInput>(tokens.clone())?;
          let fields = parsed.fields.named.into_iter().collect::<Vec<_>>();
          authority_info = Some(crate::macros::authority::extract_authority_fields(&fields)?);
          break;
        }
      }
    }
  }

  if let Some(auth_info) = authority_info {
    let total_fields = auth_info.authority_fields_required.len() + auth_info.authority_fields_optional.len();
    if total_fields > 0 {
      return Ok(Some(crate::macros::authority::generate_authority_code(&auth_info)));
    }
  }

  Ok(None)
}
