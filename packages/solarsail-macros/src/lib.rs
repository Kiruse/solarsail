use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Ident};
use syn::spanned::Spanned;

use crate::macros::{contract::ContractMode, modulator::ItemModulator};
use crate::parsers::Order;

mod macros;
mod parsers;

#[proc_macro_attribute]
pub fn contract(args: TokenStream, input: TokenStream) -> TokenStream {
  let args = proc_macro2::TokenStream::from(args);
  let mode = if !args.is_empty() {
    let span = args.span();
    match syn::parse2::<Ident>(args) {
      Ok(ident) => ContractMode::from(ident),
      Err(_) => {
        return syn::Error::new(span, "Invalid contract mode").to_compile_error().into();
      },
    }
  } else {
    ContractMode::Full
  };
  crate::macros::contract::contract(mode, &parse_macro_input!(input as syn::ItemMod)).into()
}

#[proc_macro]
pub fn state(input: TokenStream) -> TokenStream {
  crate::macros::state::state(&parse_macro_input!(input as crate::macros::state::StateMacroInput)).into()
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
/// let state = retrieve!()?;
/// ```
///
/// OR
///
/// ```rust
/// let State { total_supply, .. } = retrieve!()?;
/// ```
///
/// OR for map items:
///
/// ```rust
/// let balance = retrieve!(balances[address])?;
/// ```
#[proc_macro]
pub fn retrieve(input: TokenStream) -> TokenStream {
  macros::state::retrieve(&parse_macro_input!(input as parsers::Retrieve)).into()
}

/// Write the new state to the storage. Requires the entire `State` struct.
///
/// ```rust
/// persist!({
///   total_supply: 1000000,
/// })?;
/// ```
///
/// OR for named stores:
///
/// ```rust
/// persist!(MyStore = new_value)?;
/// ```
///
/// OR for struct construction:
///
/// ```rust
/// persist!(LogoState {
///   logo: Some(logo),
/// })?;
/// ```
///
/// OR for map items:
///
/// ```rust
/// persist!(balances[address] = new_balance)?;
/// ```
#[proc_macro]
pub fn persist(input: TokenStream) -> TokenStream {
  macros::state::persist(&parse_macro_input!(input as parsers::Persist)).into()
}

/// Update the state. Requires a list of key-value pairs, and implicitly receives the `old` state.
///
/// ```rust
/// upstate!({
///   total_supply: old.total_supply + amount,
/// })?;
/// ```
///
/// OR for custom store names:
///
/// ```rust
/// upstate!(TOKEN_INFO: {
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
  macros::state::upstate(&parse_macro_input!(input as parsers::UpState)).into()
}

/// Enumerate items in a map with optional range and ordering.
///
/// ```rust
/// enumerate!(balances);
/// enumerate!(balances[user], None..None, descending);
/// enumerate!(allowances[owner, spender], min..max);
/// enumerate!(tokens.owner[owner], min_token_id..=max_token_id);
/// ```
///
/// The first expression consists of the map name and optional prefixes. The prefixes are enclosed
/// in brackets.
///
/// `min` and `max` are optional and can be omitted. When present, they are assumed to be `Option`s.
/// When omitted, they are equivalent to `None`.
///
/// `descending` is an optional keyword to sort the items in descending order. When omitted, items
/// are sorted in ascending order.
///
/// The order in which expressions are listed does not matter, except for the map & prefixes.
#[proc_macro]
pub fn enumerate(input: TokenStream) -> TokenStream {
  macros::state::enumerate(&parse_macro_input!(input as parsers::Enumerate)).into()
}

/// Delete a storage item from a map, or an entire store.
///
/// ```rust
/// delete!(named_store);
/// delete!(balances[address]);
/// delete!(operators[(owner, operator)]);
/// ```
#[proc_macro]
pub fn delete(input: TokenStream) -> TokenStream {
  macros::state::delete(&parse_macro_input!(input as parsers::Delete)).into()
}

#[proc_macro_attribute]
pub fn modulate(args: TokenStream, input: TokenStream) -> TokenStream {
  let modulator_ident = parse_macro_input!(args as Ident);
  let item = parse_macro_input!(input as syn::ItemFn);
  crate::macros::modulate::modulate(&item, modulator_ident).into()
}

#[proc_macro]
pub fn modulator(input: TokenStream) -> TokenStream {
  let item = parse_macro_input!(input as ItemModulator);
  crate::macros::modulator::modulator(&item).into()
}

/// Assert a condition and return an error if it fails.
///
/// ```rust
/// assert!(amount > 0, ContractError::InvalidAmount { amount })?;
/// ```
#[proc_macro]
pub fn assert(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as parsers::Assert);

  let condition = parsed.condition;
  let error = parsed.error;

  quote! {
    if !(#condition) {
      return Err(#error);
    }
  }.into()
}

/// Invoke a contract with a message. Can only be used within @execute functions.
/// Creates a SubMsg and adds it to the submsgs vector.
///
/// ```rust
/// invoke!(recipient, msg)?;
/// ```
#[proc_macro]
pub fn invoke(input: TokenStream) -> TokenStream {
  let input_str = input.to_string();
  let parts: Vec<&str> = input_str.split(',').collect();

  if parts.len() != 2 {
    return syn::Error::new(
      proc_macro2::Span::call_site().into(),
      "invoke! macro expects exactly 2 arguments: recipient and msg"
    ).to_compile_error().into();
  }

  let recipient = parts[0].trim();
  let msg = parts[1].trim();

  // Parse the recipient and msg as expressions
  let recipient_expr = syn::parse_str::<syn::Expr>(recipient).unwrap_or_else(|_| {
    syn::parse_str::<syn::Expr>("recipient").unwrap()
  });

  let msg_expr = syn::parse_str::<syn::Expr>(msg).unwrap_or_else(|_| {
    syn::parse_str::<syn::Expr>("msg").unwrap()
  });

  quote! {
    {
      let submsg = cosmwasm_std::SubMsg::new(
        cosmwasm_std::WasmMsg::Execute {
          contract_addr: #recipient_expr.to_string(),
          msg: #msg_expr,
          funds: vec![],
        }
      );
      __solarsail_submsgs.push(submsg);
      Ok::<(), cosmwasm_std::StdError>(())
    }
  }.into()
}

#[proc_macro]
pub fn emit(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as parsers::EmitEvent);
  quote! { #parsed }.into()
}
