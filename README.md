# Solarsail
Solarsail is a framework for [CosmWasm](https://cosmwasm.com/) smart contracts.

## Example
Following is part of a CW20 implemented using the Solarsail framework:

```rust
use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
  #[error("{0}")]
  Std(#[from] StdError),

  #[error("Unauthorized")]
  Unauthorized,
}

pub type ContractResult<T> = std::result::Result<T, ContractError>;

#[solarsail::contract]
pub mod contract {
  use cosmwasm_schema::cw_serde;
  use cosmwasm_std::{Response, Uint128};
  use solarsail::*;
  use super::ContractError;

  state!({
    #[authority]
    minter: String,

    #[authority]
    marketing: String,
  });

  state_map!(balances   = String => Uint128);
  state_map!(allowances = (String, String) => Allowance);

  #[cw_serde]
  pub struct Allowance {
    pub amount: Uint128,
    pub expiry: Option<Expiry>,
  }

  #[cw_serde]
  pub enum Expiry {
    #[default]
    Never,
    AtTimestamp(Uint64),
    AtBlockHeight(Uint128),
  }

  #[execute]
  #[modulate(minter_only)]
  fn mint(ctx: &mut ExecuteContext, amount: Uint128, recipient: Addr) -> ContractResult<()> {
    upstate!({
      total_supply: old.total_supply + amount,
    });

    let old_balance = rstate!(balances[recipient])?;
    // all state macros can take a given store, but default to the `state!`-defined store
    wstate!(balances[recipient], old_balance + amount)?;

    Ok(Response::new())
  }

  #[execute]
  fn increase_allowance(
    ctx: &mut ExecuteContext,
    spender: Addr,
    amount: Uint128,
    expiry: Option<Expiry>,
  ) -> ContractResult<()> {
    // just like wstate!, upstate! can take a target store
    upstate!(allowances[spender], {
      amount: old.amount + amount,
      expiry: expiry,
    })?;
  }

  modulator!(minter_only {
    let minter = read_state!(minter);
    if ctx.info.sender != minter {
      return Err(ContractError::Unauthorized);
    }
    _; // modulated code goes here, like a Solidity modifier
  });
}
```

## State Management
In *Solarsail*, state management abstraction revolves around reducing boilerplate code.

There are currently 4 types of store types and corresponding macros:

- `state!` defines the global [`Item`](https://github.com/CosmWasm/cw-storage-plus?tab=readme-ov-file#item) singleton. While CosmWasm supports multiple independent global state stores, the most common use case is to have just a single. This will typically store contract-wide data such as owner or total supply.
- `state_map!` defines a [`Map`](https://github.com/CosmWasm/cw-storage-plus?tab=readme-ov-file#map). Unlike the underlying structures, the `state_map!` macro reduces some repetition and introduces some domain-specific language reminiscent of mappings in Solidity.
- `state_indexed!` defines an [`IndexedMap`](https://github.com/CosmWasm/cw-storage-plus?tab=readme-ov-file#indexedmap). This is not implemented yet. The original `IndexedMap` is a comparatively complex structure, so I would like to take my time to get the DSL for this one right.
- `state_deque!` defines a [`Deque`](https://github.com/CosmWasm/cw-storage-plus?tab=readme-ov-file#deque). It is a rather simple store type without much sugar.

### Authority
A common use case is to define an address that has some authority over a limited portion of the smart contract, such as the minter or marketing addresses.

Within the `state!` definition, the `#[authority]` attribute can be applied to `Addr`s or `String`s only. It causes the `state!` macro to generate additional code which allows only the respective address to change this authority address.

### Read, Write, Update State
***TODO***

## Modulators
Inspired by [Solidity's function modifiers](https://solidity-by-example.org/function-modifier/), *Modulators* allow you to write a wrapper function that can be applied to `#[execute]` handlers. These are useful for frequent patterns such as authorization handlers. It effectively allows decorating a handler with pre/post conditions, and allows you to see these at a glance without inspecting the function body, aiding in resilience.

Preconditions are the most common usage:

```rust
use solarsail::*;

modulator!(minter_only {
  let minter = rstate!()?.minter;
  if ctx.info.sender != minter {
    return Err(ContractError::Unauthorized);
  }
});
```

You can then apply these guards with the `#[modulate]` attribute macro:

```rust
#[execute]
#[modulate(minter_only)]
fn mint(
  ctx: &ExecuteContext,
  amount: Uint128,
  recipient: Addr,
) -> Result<Response, ContractError> {
  // ...
  Ok(Resonse::new())
}
```

For postconditions, inspired by Solidity, you use the `_;` token to split the code:

```rust
use solarsail::*;

modulator!(progress_guard {
  upstate!({ in_progress: true })?;
  _;
  upstate!({ in_progress: false })?;
});
```

Frankly, the above is not very useful, as reentrancy in CosmWasm works entirely differently than in Solidity. The above example only serves as demonstration of the `_;` splitter token.
