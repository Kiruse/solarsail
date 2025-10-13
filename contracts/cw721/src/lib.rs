//! WORK IN PROGRESS
//! NOTE: The CW721 spec is currently a living document which regularly undergoes fairly comprehensive changes.
//! Therefore, this implementation is not a 1-to-1 implementation of the standard implementation, but aims to
//! be compatible with the public/non-privileged interface

pub mod types;

#[solarsail::contract]
pub mod contract {
  use cosmwasm_std::Addr;
  use solarsail::*;

  use crate::types::*;

  state!({
    #[authority]
    creator: Option<Addr>,

    #[authority]
    minter: Option<Addr>,

    total_supply: u64,
  });

  state!(CollectionInfo, {
    name: String,
    symbol: String,
  });

  state_map!(tokens: String => Token, [owner: Addr]);

  pub fn instantiate(
    ctx: ExecuteContext,
    name: String,
    symbol: String,
  ) -> ContractResult<ContractError> {
    persist!(CollectionInfo {
      name,
      symbol,
    })?;

    Ok(())
  }

  #[contract(execute)]
  pub mod execute {
    use cosmwasm_std::{Binary, to_json_binary};
    use cw_utils::Expiration;

    use super::*;

    #[execute]
    pub fn transfer_authority(ctx: ExecuteContext, authority: TransferAuthority) -> ContractResult<ContractError> {
      super::execute_transfer_authority(ctx, authority)?;
      Ok(())
    }

    #[execute]
    #[authority(creator)]
    pub fn update_collection_info(
      ctx: ExecuteContext,
      name: String,
      symbol: String,
    ) -> ContractResult<ContractError> {
      upstate!(CollectionInfo: {
        name,
        symbol,
      })?;
      Ok(())
    }

    #[execute]
    pub fn transfer_nft(ctx: ExecuteContext, recipient: Addr, token_id: String) -> ContractResult<ContractError> {
      let mut token = retrieve!(tokens[token_id.clone()])?;
      token.check_owner(&ctx)?;
      token.owner = recipient.clone();
      persist!(tokens[token_id.clone()] = token)?;
      emit!("transfer", { recipient, token_id });
      Ok(())
    }

    #[execute]
    pub fn send_nft(ctx: ExecuteContext, recipient: Addr, token_id: String, msg: Binary) -> ContractResult<ContractError> {
      let mut token = retrieve!(tokens[token_id.clone()])?;
      token.check_owner(&ctx)?;

      token.owner = recipient.clone();
      persist!(tokens[token_id.clone()] = token)?;

      let msg = to_json_binary(&Cw721ReceiverExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
        sender: ctx.info.sender.clone(),
        token_id: token_id.clone(),
        msg,
      }))?;
      invoke!(recipient.clone(), msg)?;

      emit!("send", { recipient, token_id });
      Ok(())
    }

    #[execute]
    pub fn approve(ctx: ExecuteContext, spender: Addr, token_id: String, expires: Option<Expiration>) -> ContractResult<ContractError> {
      let mut token = retrieve!(tokens[token_id.clone()])?;
      token.check_owner(&ctx)?;
      token.approve(&ctx, spender.clone(), expires)?;
      persist!(tokens[token_id.clone()] = token)?;
      emit!("approve", { owner: ctx.info.sender, spender, token_id });
      Ok(())
    }

    #[execute]
    pub fn approve_all(ctx: ExecuteContext, operator: Addr, _expires: Option<Expiration>) -> ContractResult<ContractError> {
      let owner = &ctx.info.sender;
      let tokens = enumerate!(tokens.owner[owner.clone()])
        .collect::<Result<Vec<_>, _>>()?;
      for (token_id, mut token) in tokens {
        token.approve(&ctx, operator.clone(), _expires)?;
        persist!(tokens[token_id.clone()] = token)?;
      }
      emit!("approve.all", { owner, operator });
      Ok(())
    }

    #[execute]
    pub fn revoke(ctx: ExecuteContext, token_id: String, spender: Addr) -> ContractResult<ContractError> {
      let mut token = retrieve!(tokens[token_id.clone()])?;
      token.check_owner(&ctx)?;
      token.revoke_approval(&ctx, spender.clone())?;
      persist!(tokens[token_id.clone()] = token)?;
      emit!("revoke", { owner: ctx.info.sender, token_id, spender });
      Ok(())
    }

    #[execute]
    pub fn revoke_all(ctx: ExecuteContext, operator: Addr) -> ContractResult<ContractError> {
      let owner = &ctx.info.sender;
      let tokens = enumerate!(tokens.owner[owner.clone()])
        .collect::<Result<Vec<_>, _>>()?;
      for (token_id, mut token) in tokens {
        token.revoke_approval(&ctx, operator.clone())?;
        persist!(tokens[token_id.clone()] = token)?;
      }
      emit!("revoke.all", { owner, operator });
      Ok(())
    }

    #[execute]
    pub fn mint(
      ctx: ExecuteContext,
      recipient: Addr,
      token_id: String,
      token_uri: String,
    ) -> ContractResult<ContractError> {
      let token = Token {
        owner: recipient.clone(),
        approvals: vec![],
        token_uri,
      };
      persist!(tokens[token_id.clone()] = token)?;
      upstate!({ total_supply: old.total_supply + 1 })?;
      emit!("mint", { recipient, token_id });
      Ok(())
    }

    #[execute]
    pub fn burn(ctx: ExecuteContext, token_id: String) -> ContractResult<ContractError> {
      let mut token = retrieve!(tokens[token_id.clone()])?;
      token.check_owner(&ctx)?;
      token.owner = Addr::unchecked("");
      persist!(tokens[token_id.clone()] = token)?;
      upstate!({ total_supply: old.total_supply - 1 })?;
      emit!("burn", { token_id });
      Ok(())
    }
  }
}
