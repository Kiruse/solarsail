use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

use crate::macros::modulator::ItemModulator;

mod macros;
mod parsers;

#[proc_macro_attribute]
pub fn contract(_args: TokenStream, input: TokenStream) -> TokenStream {
  crate::macros::contract::contract(&parse_macro_input!(input as syn::ItemMod)).into()
}

#[proc_macro]
pub fn state(input: TokenStream) -> TokenStream {
  crate::macros::state::state(&parse_macro_input!(input as syn::FieldsNamed)).into()
}

/// Define a Map for key-value storage.
///
/// ```rust
/// state_map!(balances = String => Uint128);
/// state_map!(allowances = (String, String) => Allowance);
/// ```
#[proc_macro]
pub fn state_map(input: TokenStream) -> TokenStream {
  crate::macros::state::state_map(&parse_macro_input!(input as parsers::StateMap)).into()
}

/// Read the current state from the storage.
///
/// ```rust
/// let state = rstate!()?;
/// ```
///
/// OR
///
/// ```rust
/// let State { total_supply, .. } = rstate!()?;
/// ```
///
/// OR for map items:
///
/// ```rust
/// let balance = rstate!(balances[address])?;
/// ```
#[proc_macro]
pub fn rstate(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as parsers::RState);

  match (parsed.map_name, parsed.item_name) {
    // Map item variant
    (Some(map_name), Some(item_name)) => {
      let const_name = syn::Ident::new(
        &map_name.to_string().to_uppercase(),
        map_name.span(),
      );
      quote! {
        #const_name.load(ctx.deps.storage, #item_name)
      }.into()
    }
    // Global state variant
    (None, None) => {
      quote! {
        STATE.load(ctx.deps.storage)
      }.into()
    }
    _ => {
      // This should never happen with our parser
      quote! {
        compile_error!("Invalid rstate syntax")
      }.into()
    }
  }
}

/// Write the new state to the storage. Requires the entire `State` struct.
///
/// ```rust
/// wstate!({
///   total_supply: 1000000,
/// })?;
/// ```
///
/// OR for map items:
///
/// ```rust
/// wstate!(balances[address], new_balance)?;
/// ```
#[proc_macro]
pub fn wstate(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as parsers::WState);

  match (parsed.map_name, parsed.item_name) {
    (Some(map_name), Some(item_name)) => {
      // Map item variant
      let const_name = syn::Ident::new(
        &map_name.to_string().to_uppercase(),
        map_name.span(),
      );
      let value = parsed.value;
      quote! {
        #const_name.save(ctx.deps.storage, #item_name, &#value)
      }.into()
    }
    (None, None) => {
      // Global state variant
      let value = parsed.value;
      quote! {
        STATE.save(ctx.deps.storage, &#value)
      }.into()
    }
    _ => {
      // This should never happen with our parser
      quote! {
        compile_error!("Invalid wstate syntax")
      }.into()
    }
  }
}

/// Update the state. Requires a list of key-value pairs, and implicitly receives the `old` state.
///
/// ```rust
/// upstate!({
///   total_supply: old.total_supply + amount,
/// })?;
/// ```
///
/// OR for map items:
///
/// ```rust
/// upstate!(balances[address], {
///   amount: old.amount + value,
/// })?;
/// ```
#[proc_macro]
pub fn upstate(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as parsers::UpState);

  match (parsed.map_name, parsed.item_name) {
    (Some(map_name), Some(item_name)) => {
      // Map item variant
      let const_name = syn::Ident::new(
        &map_name.to_string().to_uppercase(),
        map_name.span(),
      );
      let pairs = parsed.kvs.pairs.iter().map(|(key, value)| {
        quote! {
          #key: #value,
        }
      }).collect::<Vec<_>>();
      quote! {
        #const_name.update(ctx.deps.storage, #item_name, |old| -> Result<_, cosmwasm_std::StdError> {
          Ok(Item {
            #(#pairs),*
            ..old
          })
        })
      }.into()
    }
    (None, None) => {
      // Global state variant
      let pairs = parsed.kvs.pairs.iter().map(|(key, value)| {
        quote! {
          #key: #value,
        }
      }).collect::<Vec<_>>();
      quote! {
        STATE.update(ctx.deps.storage, |old| -> Result<_, cosmwasm_std::StdError> {
          Ok(State {
            #(#pairs),*
            ..old
          })
        })
      }.into()
    }
    _ => {
      // This should never happen with our parser
      quote! {
        compile_error!("Invalid upstate syntax")
      }.into()
    }
  }
}

#[proc_macro_attribute]
pub fn execute(args: TokenStream, input: TokenStream) -> TokenStream {
  // Parse args as a list of key = value pairs
  let args_parsed = parse_macro_input!(args as parsers::AssignPairs);
  let item = parse_macro_input!(input as syn::ItemFn);

  crate::macros::execute::execute(&item, args_parsed.pairs).into()
}

#[proc_macro_attribute]
pub fn modulate(args: TokenStream, input: TokenStream) -> TokenStream {
  let modulator_ident = parse_macro_input!(args as syn::Ident);
  let item = parse_macro_input!(input as syn::ItemFn);
  crate::macros::modulate::modulate(&item, modulator_ident).into()
}

#[proc_macro]
pub fn modulator(input: TokenStream) -> TokenStream {
  let item = parse_macro_input!(input as ItemModulator);
  crate::macros::modulator::modulator(&item).into()
}
