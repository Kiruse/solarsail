use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::parse_macro_input;

use crate::parsers::Order;

mod macros;
mod parsers;

#[proc_macro]
pub fn contract(input: TokenStream) -> TokenStream {
  let res = crate::macros::contract::contract(parse_macro_input!(input as parsers::ContractDef));
  match res {
    Ok(expanded) => expanded.into(),
    Err(e) => e.to_compile_error().into(),
  }
}

/// The `solarize` macro is a multi-purpose macro that applies to various code structures:
///
/// ## `#[solarize(execute|query)] impl`
/// When applied to an `impl` block, it can be used to define the execution & query messages and
/// their handlers. Any `pub fn` will become a part of the contract's public interface; the other
/// functions will become a part of its internal interface.
///
/// ```rust
/// #[solarize(execute)]
/// impl Cw20Contract {
///     fn mint(ctx: ExecuteContext, amount: Uint128, recipient: Addr) -> ExecuteResult<ContractError> {
///         // ... implement mint handler here
///     }
///
///     pub fn transfer(ctx: ExecuteContext, amount: Uint128, recipient: Addr) -> ExecuteResult<ContractError> {
///         // ... implement transfer handler here
///     }
/// }
/// ```
///
/// ```rust
/// #[solarize(query)]
/// impl Cw20Contract {
///     #[returns({
///         balance: Uint128,
///     })]
///     pub fn balance_of(ctx: QueryContext, address: Addr) -> Result<_, ContractError> {
///         let balance = retrieve!(Balances[address])?;
///         Ok(respond! { balance })
///     }
/// }
/// ```
///
/// ## `#[solarize] enum|struct`
/// When applied to an `enum` or a `struct` block, it can be used to transform the type into a
/// compatible network type, i.e. a message. You don't have to use this macro, but it's convenient.
/// If you choose not to, you will need to manually implement serde traits.
///
/// ```rust
/// #[solarize]
/// pub struct TokenInfo {
///     pub name: String,
///     pub symbol: String,
///     pub description: String,
///     pub decimals: u8,
///     pub cap: Option<Uint128>,
/// }
/// ```
///
/// ## `#[solarize] fn instantiate|migrate`
/// When applied to an `fn instantiate` or `fn migrate`, these two specially designated methods are
/// transformed into their respective contract entrypoints & handlers.
///
/// ```rust
/// #[solarize]
/// fn instantiate(
///     ctx: ExecuteContext,
///     name: String,
///     symbol: String,
///     description: String,
///     decimals: u8,
///     cap: Option<Uint128>,
/// ) -> ExecuteResult<ContractError> {
///     persist!(TokenInfo { name, symbol, description, decimals, cap })?;
///     Ok(())
/// }
/// ```
///
/// ```rust
/// #[solarize]
/// fn migrate(ctx: ExecuteContext) -> ExecuteResult<ContractError> {
///     // add whatever storage migration logic you need here.
///     // migration will receive further abstractions in the future to aid in ensuring
///     // the migration is proper.
///     // if no storage migration is necessary, just return `Ok(())`. the presence of
///     // this method makes a contract migrateable; without it, you cannot migrate.
///     Ok(())
/// }
/// ```
#[proc_macro_attribute]
pub fn solarize(args: TokenStream, input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as parsers::SolarizePrep);
  match solarize_impl(args, input) {
    Ok(expanded) => expanded.into(),
    Err(e) => e.to_compile_error().into(),
  }
}

fn solarize_impl(args: TokenStream, input: parsers::SolarizePrep) -> Result<proc_macro2::TokenStream, syn::Error> {
  match input {
    parsers::SolarizePrep::Fn(func) => {
      if !args.is_empty() {
        Err(syn::Error::new(Span::call_site(), "solarize can only be applied to `instantiate` or `migrate` functions"))
      } else {
        match func.sig.ident.to_string().as_str() {
          "instantiate" =>
            crate::macros::instantiate::transform(&func),
          "migrate" =>
            crate::macros::migrate::transform(&func),
          _ => Err(syn::Error::new(func.sig.ident.span(), "Unknown solarize fn kind"))
        }
      }
    }
    parsers::SolarizePrep::Impl(item) => {
      let args = syn::parse(args)?;
      crate::macros::solarize::transform_impl(args, item)
    }
    parsers::SolarizePrep::Ty(ty) => {
      crate::macros::solarize::transform_ty(ty)
    }
  }
}

#[proc_macro_attribute]
pub fn authority(args: TokenStream, input: TokenStream) -> TokenStream {
  let authority = parse_macro_input!(args as parsers::AuthorityArgs);
  let item = parse_macro_input!(input as syn::ItemFn);
  match crate::macros::authority::transform(item, &authority.authorities.iter().collect::<Vec<_>>()) {
    Ok(expanded) => expanded.into(),
    Err(e) => e.to_compile_error().into(),
  }
}

#[proc_macro]
pub fn response(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as parsers::Response);
  quote! { SSAnonymousResponse #parsed }.into()
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
  let parsed = parse_macro_input!(input as parsers::Invoke);
  quote! { #parsed }.into()
}

#[proc_macro]
pub fn emit(input: TokenStream) -> TokenStream {
  let parsed = parse_macro_input!(input as parsers::EmitEvent);
  quote! { #parsed }.into()
}
