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
#[proc_macro]
pub fn rstate(input: TokenStream) -> TokenStream {
  parse_macro_input!(input as syn::parse::Nothing);
  quote! {
    STATE.load(ctx.deps.storage)
  }.into()
}

/// Write the new state to the storage. Requires the entire `State` struct.
///
/// ```rust
/// wstate!({
///   total_supply: 1000000,
/// })?;
/// ```
#[proc_macro]
pub fn wstate(input: TokenStream) -> TokenStream {
  let expr = parse_macro_input!(input as syn::Expr);
  quote! {
    STATE.save(ctx.deps.storage, &#expr)
  }.into()
}

/// Update the state. Requires a list of key-value pairs, and implicitly receives the `old` state.
///
/// ```rust
/// upstate!({
///   total_supply: old.total_supply + amount,
/// })?;
/// ```
#[proc_macro]
pub fn upstate(input: TokenStream) -> TokenStream {
  let kvs = parse_macro_input!(input as parsers::KVPairs);
  let pairs = kvs.pairs.iter().map(|(key, value)| {
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
