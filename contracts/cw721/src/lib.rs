//! WORK IN PROGRESS
//! NOTE: The CW721 spec is currently a living document which regularly undergoes fairly comprehensive changes.
//! Therefore, this implementation is not a 1-to-1 implementation of the standard implementation, but aims to
//! be compatible with the public/non-privileged interface

pub mod types;

#[solarsail::contract]
pub mod contract {
  use cosmwasm_std::Addr;
  use cw_utils::Expiration;
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
  state_map!(operators: (Addr, Addr) => Expiration);

  error!(NotFound);

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
      persist!(operators[(owner.clone(), operator.clone())] = _expires.unwrap_or(Expiration::Never {}))?;
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
      delete!(operators[(owner.clone(), operator.clone())]);
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

  #[contract(query)]
  pub mod query {
    use cosmwasm_schema::cw_serde;

    use crate::types::Approval;

    use super::*;

    #[cw_serde]
    pub struct OwnerOfResponse {
      pub owner: Addr,
      pub approvals: Vec<Approval>,
    }

    #[query]
    pub fn owner_of(ctx: QueryContext, token_id: String) -> Result<OwnerOfResponse, ContractError> {
      let token = retrieve!(tokens[token_id.clone()])?;
      Ok(OwnerOfResponse {
        owner: token.owner,
        approvals: token.approvals,
      })
    }

    #[cw_serde]
    pub struct ApprovalResponse {
      pub approval: Approval,
    }

    #[query]
    pub fn approval(ctx: QueryContext, token_id: String, spender: Addr, include_expired: Option<bool>) -> Result<ApprovalResponse, ContractError> {
      let token = retrieve!(tokens[token_id.clone()])?;

      // NFT owner always has approval
      if spender == token.owner {
        return Ok(ApprovalResponse {
          approval: Approval {
            spender,
            expires: Expiration::Never {},
          },
        });
      }

      // Explicit approval
      let approval = token.approvals
        .iter()
        .filter(|a| a.spender == spender)
        .filter(|a| include_expired.unwrap_or(false) || !a.expires.is_expired(&ctx.env.block))
        .next();
      if let Some(approval) = approval {
        return Ok(ApprovalResponse {
          approval: approval.clone(),
        });
      }

      // Implicit approval via token owner's operators
      let expires = retrieve!(operators[(token.owner, spender.clone())])
        .map_err(|_| ContractError::NotFound)
        .and_then(|x| if x.is_expired(&ctx.env.block) { Err(ContractError::NotFound) } else { Ok(x) })?;
      Ok(ApprovalResponse {
        approval: Approval {
          spender,
          expires,
        },
      })
    }

    #[cw_serde]
    pub struct ApprovalsResponse {
      pub approvals: Vec<Approval>,
    }

    #[query]
    fn approvals(ctx: QueryContext, token_id: String, include_expired: Option<bool>) -> Result<ApprovalsResponse, ContractError> {
      let token = retrieve!(tokens[token_id.clone()])?;
      let explicit = token.approvals
        .iter()
        .filter(|a| include_expired.unwrap_or(false) || !a.expires.is_expired(&ctx.env.block))
        .cloned();
      let implicit = enumerate!(operators[token.owner])
        .filter(|op| op.is_ok())
        .map(|op| op.unwrap())
        .filter(|(_, expires)| include_expired.unwrap_or(false) || !expires.is_expired(&ctx.env.block))
        .map(|(spender, expires)| Approval {
          spender,
          expires,
        });
      Ok(ApprovalsResponse {
        approvals: explicit.chain(implicit).collect(),
      })
    }
  }
}
