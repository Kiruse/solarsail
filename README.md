# Solarsail
Solarsail is a framework for [CosmWasm](https://cosmwasm.com/) smart contracts.

**Attention!** Solarsail is currently in `v0.1.0`, meaning it is early-stage, unstable, and rapidly evolving. Use at your own risk!

## Installation
Solarsail is not yet published to crates.io. For now, add Solarsail to your `Cargo.toml` as a git dependency:

```toml
[dependencies]
solarsail = { git = "https://github.com/Kiruse/solarsail", package = "solarsail" }
```

## Example
```rust
use cosmwasm_std::*;
use solarsail::*;
// This crate does not currently exist, but we'll pretend it does for the example.
use solarsail_cw20::{Cw20Contract, Cw20ReceiverExecuteMsg, Cw20ReceiveMsg};

// DSL for contract-wide declarations, such as name, inheritance, permissioned authorities, state
// items, state maps, and errors. It generates a bunch of code that would otherwise require a lot of
// boilerplate. In this case, it generates the `struct Cw20ExtraContract` which will be further
// declared below.
contract! {
  name! Cw20Extra;
  authority! [minter, marketing];

  // You can extend exactly one base solarsail contract. Your contract will "steal" the base contract's
  // public & internal interfaces + their implementations, and you can override handlers to suit your
  // needs.
  extends! Cw20Contract;

  // You can implement however many contract interfaces as you want. Implementations are based on
  // Rust traits. These allow you to easily specify compatibility with other smart contracts. However,
  // as messaging uses serialized data structures, you must be sure that the messages of each interface
  // is unique and does not collide with the others!
  implements! [SomeContractInterfaceA, SomeContractInterfaceB];

  state! TokenInfo {
    name: String,
    symbol: String,
    decimals: u8,
    cap: Option<Uint128>,
  }

  state! Balances  : String => Uint128;
  state! Allowances: (String, String) => Allowance;

  error! InsufficientBalance : "Insufficient balance";
  error! Overflow(#[from] cosmwasm_std::Overflow) : "Overflow: {0}";
}

#[solarize(execute)]
impl Cw20ExtraContract {
  #[authority(minter)]
  fn mint(mut ctx: ExecuteContext, recipient: Addr, amount: Uint128) -> ExecuteResult<Cw20ExtraError> {
    upstate!(Balances[recipient], old.checked_add(amount)?)?;
    emit!("mint", { recipient, amount });
    Ok(())
  }

  pub fn transfer(mut ctx: ExecuteContext, recipient: Addr, amount: Uint128) -> ExecuteResult<Cw20ExtraError> {
    self.transfer_internal(&mut ctx, recipient, amount)?;
    emit!("transfer", { sender, recipient, amount });
    Ok(())
  }

  pub fn send(mut ctx: ExecuteContext, recipient: Addr, amount: Uint128, msg: Binary) -> ExecuteResult<Cw20ExtraError> {
    self.transfer_internal(ctx, recipient, amount)?;
    let msg = to_json_binary(Cw20ReceiverExecuteMsg::Receive(Cw20ReceiveMsg {
      sender: ctx.info.sender.clone(),
      amount,
      msg,
    }))?;
    invoke!(recipient, msg);
    emit!("send", { sender, recipient, amount });
    Ok(())
  }
}

#[solarize(query)]
impl Cw20Contract {
  // Inside the `#[query]` impl, you can use the `#[returns({ ... })]` attribute to define response
  // types that are only used here anyways. They will be named after your method plus the "Response"
  // suffix, so in this case `BalanceOfResponse`. The corresponding `respond!` macro can then be
  // used to construct an object of this type.
  #[returns({
    balance: Uint128,
  })]
  pub fn balance_of(ctx: QueryContext, address: Addr) -> Result<_, Cw20ExtraError> {
    let balance = retrieve!(Balances[address])?;
    Ok(respond! { balance })
  }
}

// A non-`#[query]` & non-`#[execute]` annotated `impl` block will behave as normal. These methods
// will not be included in your contract's ExecuteMsg or QueryMsg.
impl Cw20Contract {
  fn transfer_internal(ctx: &mut ExecuteContext, recipient: Addr, amount: Uint128) -> Result<(), Cw20Error> {
    let sender = ctx.info.sender.clone();
    let balance = retrieve!(Balances[sender])?;
    solarsail::assert!(balance > amount, Cw20Error::InsufficientBalance);

    persist!(Balances[sender], balance.checked_sub(amount)?)?;
    upstate!(Balances[recipient], old.checked_add(amount)?)?;
    Ok(())
  }
}
```

See the [full example here](./contracts/cw20/README.md).

Learn more about Solarsail in the official [Solarsail Book](https://solarsail.kiruse.dev).

## Roadmap
- [ ] **Contract Components:** Add composable features to your smart contract with `compose!` and `implements!`
- [ ] **Multi-VM support:** Ideally, Solarsail is capable of abstracting parts of smart contract development across multiple Rust-based smart contract VMs away. These are the VMs I'm aware of & intend to add support for in the long term:
  - [ ] **Grug support:** Grug is a VM inspired by CosmWasm, and thus very similar. Solarsail, being a macro-based DSL & code generator, can easily target Grug by adding `feature = "grug"` to your *Cargo.toml*.
  - [ ] **SVM support:** Solana's SVM also uses Rust for its smart contracts. However, it is a very different ecosystem and poses a challenge to target. This will likely take a while.
  - [ ] **ICP support:** [Internet Computer](https://internetcomputer.org) is another VM using Rust for smart contracts.
- [ ] **Formal verification:** With a specialized DSL, formal verification can be built to ascertain certain properties of your smart contracts at compile-time.
