use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary};
use cw_utils::Expiration;
use solarsail::ExecuteContext;

use crate::contract::ContractError;

#[cw_serde]
pub struct Token {
  /// Owner of the NFT. When None, the NFT is burnt.
  pub owner: Addr,
  pub approvals: Vec<Approval>,
  pub token_uri: String,
}

#[cw_serde]
pub struct Approval {
  pub spender: Addr,
  pub expires: Expiration,
}

impl Token {
  pub fn check_owner(&self, ctx: &ExecuteContext) -> Result<(), ContractError> {
    if self.owner != ctx.info.sender {
      Err(ContractError::Unauthorized)
    } else {
      Ok(())
    }
  }

  pub fn check_allowance(&self, ctx: &ExecuteContext) -> Result<(), ContractError> {
    if self.approvals.iter().any(|approval| approval.spender == ctx.info.sender && !approval.expires.is_expired(&ctx.env.block)) {
      Ok(())
    } else {
      Err(ContractError::Unauthorized)
    }
  }

  pub fn approve(&mut self, ctx: &ExecuteContext, spender: Addr, expires: Option<Expiration>) -> Result<(), ContractError> {
    self.revoke_approval(ctx, spender.clone())?;
    self.approvals.push(Approval {
      spender,
      expires: expires.unwrap_or(Expiration::Never {}),
    });
    Ok(())
  }

  pub fn revoke_approval(&mut self, ctx: &ExecuteContext, spender: Addr) -> Result<(), ContractError> {
    self.approvals.retain(|approval| approval.spender != spender && !approval.expires.is_expired(&ctx.env.block));
    Ok(())
  }
}

#[cw_serde]
pub enum Cw721ReceiverExecuteMsg {
  ReceiveNft(Cw721ReceiveMsg),
}

#[cw_serde]
pub struct Cw721ReceiveMsg {
  pub sender: Addr,
  pub token_id: String,
  pub msg: Binary,
}
