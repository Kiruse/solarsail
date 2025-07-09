use cosmwasm_std::StdError;
use thiserror::Error;

pub type ContractResult<T> = std::result::Result<T, ContractError>;

#[derive(Error, Debug)]
pub enum ContractError {
  #[error("{0}")]
  Std(#[from] StdError),

  #[error("Unauthorized")]
  Unauthorized,
}

#[solarsail::contract]
pub mod contract {
  use cosmwasm_std::{Response, Uint128};
  use solarsail::*;
  use super::{ContractError, ContractResult};

  state!({
    #[authority]
    minter: String,

    #[authority]
    marketing: String,

    total_supply: Uint128,
  });

  state_map!(balances   : String => Uint128);
  state_map!(allowances : (String, String) => Allowance);

  #[cosmwasm_schema::cw_serde]
  pub struct Allowance {
    pub amount: Uint128,
    pub expiry: Option<Expiry>,
  }

  #[cosmwasm_schema::cw_serde]
  pub enum Expiry {
    Never,
    AtTimestamp(cosmwasm_std::Uint64),
    AtBlockHeight(Uint128),
  }

  #[execute]
  #[modulate(minter_only)]
  fn mint(ctx: &ExecuteContext, amount: Uint128, recipient: String) -> ContractResult<Response> {
    // modulate(minter_only) already enforces that the sender is the minter
    let recipient = ctx.deps.api.addr_validate(&recipient)?;
    let balance = rstate!(balances[recipient.to_string()])?;
    wstate!(balances[recipient.to_string()], &(balance + amount))?;
    upstate!({ total_supply: old.total_supply + amount })?;
    Ok(Response::new())
  }

  modulator!(minter_only {
    let minter = rstate!()?.minter;
    if ctx.info.sender != minter {
      return Err(ContractError::Unauthorized);
    }
  });
}
