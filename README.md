# Solarsail
Solarsail is a framework for [CosmWasm](https://cosmwasm.com/) smart contracts.

**Attention!** Solarsail is currently in `v0.0.1`, meaning it is incredibly early, unstable, and rapidly evolving. Use at your own risk!

## Example
Following is part of a CW20 implemented using the Solarsail framework:

```rust
#[solarsail::contract]
pub mod contract {
  use cosmwasm_schema::cw_serde;
  use cosmwasm_std::{Addr, Response, Uint128};
  use solarsail::*;
  use super::ContractError;

  state!({
    #[authority]
    minter: String,

    #[authority]
    marketing: String,
  });

  state!(TokenInfo, {
    name: String,
    symbol: String,
    decimals: u8,
    total_supply: Uint128,
    cap: Option<Uint128>,
  });

  state_map!(balances   : String => Uint128);
  state_map!(allowances : (String, String) => Allowance);

  error!(InsufficientBalance, "Insufficient balance");
  error!(Overflow(#[from] cosmwasm_std::Overflow));

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

  pub fn instantiate(
    ctx: ExecuteContext,
    name: String,
    symbol: String,
    decimals: Option<u8>,
    cap: Option<Uint128>,
  ) -> ContractResult<ContractError> {
    persist!(TokenInfo {
      name,
      symbol,
      decimals: decimals.unwrap_or(6u8),
      total_supply: Uint128::zero(),
      cap,
    })?;

    Ok(())
  }

  #[execute]
  #[authority(minter)]
  pub fn mint(ctx: ExecuteContext, amount: Uint128, recipient: Addr) -> ContractResult<ContractError> {
    // upstate allows partial state updates.
    // `old` is implicitly defined within this macro to refer to the
    // existing stored state.
    upstate!(TokenInfo: {
      total_supply: old.total_supply + amount,
    });

    let old_balance = retrieve!(balances[recipient])?;
    // all state macros can take an optional target store.
    // default is the default/unnamed store.
    persist!(balances[recipient], old_balance + amount)?;

    Ok(Response::new())
  }

  #[execute]
  pub fn increase_allowance(
    ctx: ExecuteContext,
    spender: Addr,
    amount: Uint128,
    expiry: Option<Expiry>,
  ) -> ContractResult<ContractError> {
    upstate!(allowances[spender], {
      amount: old.amount + amount,
      expiry: expiry,
    })?;
  }

  #[cw_serde]
  pub struct BalanceResponse {
    pub balance: Uint128,
  }

  #[query]
  pub fn query_balance(ctx: QueryContext, address: Addr) -> Result<BalanceResponse, ContractError> {
    Ok(BalanceResponse {
      balance: retrieve!(balances[address])?,
    })
  }
}
```

See the [full example here](./contracts/cw20/README.md).

## Authority
A common use case is to define an address that has some authority over a limited portion of the smart contract, such as the minter or marketing addresses.

Within the `state!` definition, the `#[authority]` attribute can be applied to `Addr`s or `Option<Addrs>`s only. It triggers code generation to transfer & ascertain authorization. The `#[authority(Which)]` attribute can then be applied to `#[execute]` functions to automatically restrict access to the method.

The code generation automatically creates the `transfer_authority`, `check_authority` & `query_authority` functions in the contract root, and `Authority` and `TransferAuthority` enums to go along with them.

If the full contract is in the root, the `transfer_authority` and `query_authority` methods will be exposed as contract messages automatically.

```rust
#[solarsail::contract]
pub mod contract {
  use cosmwasm_std::{Addr, Uint128};
  use solarsail::*;

  state!({
    #[authority]
    minter: Option<Addr>,
  });

  pub fn instantiate(ctx: ExecuteContext) -> ContractResponse<ContractError> {
    Ok(())
  }

  // at least 1 execution is required to generate the entrypoint
  #[execute]
  #[authority(minter)]
  pub fn mint(ctx: ExecuteContext, recipient: Addr, amount: Uint128) -> ContractResponse<ContractError> {
    emit!("mint", { recipient, amount });
    Ok(())
  }
}
```

This minimal contract will support 2 executions: `mint` and `transfer_authority`.

```rust
#[solarsail::contract]
pub mod contract {
  use cosmwasm_std::{Addr, Uint128};
  use solarsail::*;

  state!({
    #[authority]
    minter: Option<Addr>,
  });

  pub fn instantiate(ctx: ExecuteContext) -> ContractResponse<ContractError> {
    Ok(())
  }

  #[contract(execute)]
  pub mod execute {
    use super::*;

    #[execute]
    #[authority(minter)]
    pub fn mint(ctx: ExecuteContext, recipient: Addr, amount: Uint128) -> ContractResponse<ContractError> {
      emit!("mint", { recipient, amount });
      Ok(())
    }

    #[execute]
    pub fn update_minter(ctx: ExecuteContext, new_minter: Option<Addr>) -> ContractResponse<ContractError> {
      execute_transfer_authority(ctx, TransferAuthority::Minter(new_minter))?;
      match new_minter {
        Some(new_minter) => emit!("minter.update", { new_minter }),
        None => emit!("minter.renounce"),
      }
      Ok(())
    }
  }
}
```

This minimal contract will also support 2 executions: `mint` and `update_minter`. The generated `execute_transfer_authority` still exists, but is not exposed. You can still call it manually. The `update_minter` function does not need the `#[authority(minter)]` attribute because the `execute_transfer_authority` method already asserts authorization, but having the attribute won't hurt either.

## Interfaces
Rust has no built-in way to enforce one enum "extends" another and thus decode the same JSON. I aim to provide such a mechanism in *Solarsail*.

# Store Abstraction
Proper state management is critical. One of my goals of this library is to build a store abstraction that simplifies contract state management and aids in avoiding state errors. However, I am yet unsure what exactly this entails.

Thus, v0 of state management will be merely convenience macros. v1 will then focus on introducing an alternative to [`cw-storage-plus`](https://crates.io/crates/cw-storage-plus) which will likely consist of a hybrid system of macros and new primitives.

## Store v0
As mentioned above, *Store v0* focuses on merely providing convenience macros. All of these support various usages:

- `state!` - declare a state store
- `state_map!` - declare a (multi-level) associative state store
- `retrieve!` - load from a state store
- `persist!` - override a state store
- `upstate!` - partially update a state store

### `state!` & `state_map!` declaration
The `state!` macro declares either the default state store or a named state store. This type of store holds contract-wide data, such as total supply of a fungible token, or the next available NFT ID in a minter contract. The macro generates both a serializable struct and a corresponding storage item constant.

**Unnamed store usage:**
```rust
state!({
  total_supply: Uint128,
  minter: Option<Addr>,
  marketing: Option<Addr>,
});
```

**Named store:**
```rust
state!(LogoState: {
  logo: Option<Logo>,
});
```

The `state_map!` macro creates associative storage for key-value pairs, supporting complex key types like tuples for multi-level mappings.

**Basic map:**
```rust
state_map!(balances: Addr => Uint128);
```

**Complex key map:**
```rust
state_map!(allowances: (Addr, Addr) => Allowance);
```

### `retrieve!` macro
The `retrieve!` macro provides a convenient way to load data from state stores. It supports both global state and map items with a clean, intuitive syntax.

**Loading unnamed store:**
```rust
let state = retrieve!()?;
let State { total_supply, .. } = retrieve!()?;
```

**Loading named store:**
```rust
let state = retrieve!(LogoState)?;
```

**Loading from maps:**
```rust
let balance = retrieve!(balances[address])?;
let allowance = retrieve!(allowances[(owner, spender)])?;
```

### `persist!` macro
The `persist!` macro overrides the entire store state.

**Unnamed store:**
```rust
persist!({
  total_supply: Uint128::from(10000000),
  minter: Some(minter_addr),
})?;

let new_state = State {
  total_supply: Uint128::from(10000000),
  minter: None,
}
persist!(new_value);
```

**Named store:**
```rust
persist!(LogoState {
  logo: Some(logo),
})?;
```

**Map item updates:**
```rust
persist!(balances[address] = new_balance)?;
persist!(allowances[(owner, spender)] = Allowance {
  amount,
  expiry: None,
})?;
```

### `upstate!` macro
The `upstate!` macro enables partial updates to existing state, automatically handling the retrieval of the current state and applying only the specified changes. This is particularly useful for atomic operations that need to modify only specific fields.

**Partial state updates:**
```rust
upstate!({
  total_supply: old.total_supply + amount,
})?;
```

**Map item partial updates:**
```rust
upstate!(balances[address], {
  amount: old.amount + value,
})?;
```

The `upstate!` macro is especially powerful because it provides access to the `old` state through destructuring, allowing you to perform calculations based on existing values while maintaining atomicity. This pattern is commonly used in token contracts for operations like minting, burning, or transferring tokens where you need to update multiple related fields atomically.

## Store v1
In Solarsail v1, the storage abstraction layer will be a complete rewrite that likely no longer relies on [cw-storage-plus](https://github.com/CosmWasm/cw-storage-plus) in favor of applying additional heuristics & biased storage management.

For example, Store v1 will apply heuristics to manage storage keys and optimize storage footprint. It will deliver new primitives to reduce logical flaws such as forgetting to commit changes to storage. And it will tie into the [Formal Verification](#formal-verification) system.

# Formal Verification
By implementing smart contracts with a holistic DSL-like macro framework, we can declare our assumptions over inputs & outputs and statically verify their accuracy. Macros in particular lend themselves toward this concept as they are closely tied to Rust's compiler and allow catching potential bugs before even running unit tests. By applying the `#[contract]` macro to an entire module, we can provide individual `#[execute]` functions with additional context on storage.

Formal Verification will be a large part of *Solarsail*. However, at this time, it has no formal verification capacity yet. It will take inspiration from existing testing suites like [proptest](https://crates.io/crates/proptest), [coq-of-rust](https://github.com/formal-land/coq-of-rust/) and [prusti](https://github.com/viperproject/prusti-dev/), but is not designed to replace these rather than complement.

# Runtime Agnosticism
I see a huge potential in a macro-based smart contract frameworks: They can be built to target different runtimes. While *Solarsail* has its roots in [Cosmos](https://cosmos.network/) & [CosmWasm](https://cosmos.network/cosmwasm), its macros intentionally resemble a [DSL](https://www.jetbrains.com/mps/concepts/domain-specific-languages/) such that these macros can be retargeted for other Rust-based smart contract VMs like Solana's SVM.
