//! NOTE: This contract is not a 1-to-1 implementation of the cw721-base contract. As that repository
//! has become very complex and generalized, this implementation focuses mainly on the public interface
//! for conformity with the CW721 standard. As Solarsail evolves, this contract will likely be split
//! into a base contract and various components that you can compose together in your own contract.
use cosmwasm_std::{Addr, Binary, to_json_binary};
use cw_utils::Expiration;
use solarsail::{scaffold::ExecuteResult, *};

pub mod types;
pub use types::*;

contract! {
  name! Cw721;

  authority! [creator, minter];

  state! CollectionInfo {
    name: String,
    symbol: String,
    description: Option<String>,
    /// Current total supply of NFTs in this collection.
    total_supply: u64,
    /// Hard limit on the total supply of NFTs in this collection, if any.
    cap: Option<u64>,
  }

  state! tokens: String => Token, [owner: Addr];
  state! token_metadata: String => TokenMetadata;
  state! operators: (Addr, Addr) => Expiration;

  error! NotFound;
}

#[solarize]
fn instantiate(
  ctx: &mut ExecuteContext,
  name: String,
  symbol: String,
  description: Option<String>,
  cap: Option<u64>,
) -> ExecuteResult<Cw721Error> {
  persist!(CollectionInfo {
    name,
    symbol,
    description,
    cap,
    total_supply: 0,
  })?;
  Ok(())
}

#[solarize]
fn migrate(ctx: &mut ExecuteContext) -> ExecuteResult<Cw721Error> {
  Ok(())
}

#[solarize(execute)]
impl Cw721Contract {
  fn transfer_authority(ctx: &mut ExecuteContext, #[msg] op: AuthorityOperation) -> Result<(), Cw721Error> {
    op.handle(ctx)?;
    Ok(())
  }

  #[authority(creator)]
  fn update_collection_info(ctx: &mut ExecuteContext, name: String, symbol: String) -> Result<(), ContractError> {
    upstate!(CollectionInfo: {
      name,
      symbol,
    })?;
    Ok(())
  }

  pub fn transfer_nft(ctx: &mut ExecuteContext, recipient: Addr, token_id: String) -> Result<(), Cw721Error> {
    let mut token = retrieve!(tokens[token_id.clone()])?;
    token.check_owner(&ctx)?;
    token.owner = recipient.clone();
    persist!(tokens[token_id.clone()] = token)?;
    emit!("transfer", { recipient, token_id });
    Ok(())
  }

  pub fn send_nft(ctx: &mut ExecuteContext, recipient: Addr, token_id: String, msg: Binary) -> Result<(), Cw721Error> {
    let mut token = retrieve!(tokens[token_id.clone()])?;
    token.check_owner(&ctx)?;

    token.owner = recipient.clone();
    persist!(tokens[token_id.clone()] = token)?;

    let msg = to_json_binary(&Cw721ReceiverExecuteMsg::ReceiveNft(Cw721ReceiveMsg {
      sender: ctx.info.sender.clone(),
      token_id: token_id.clone(),
      msg,
    }))?;
    invoke!(recipient.clone(), msg);

    emit!("send", { recipient, token_id });
    Ok(())
  }

  pub fn approve(ctx: &mut ExecuteContext, spender: Addr, token_id: String, expires: Option<Expiration>) -> Result<(), Cw721Error> {
    let owner = &ctx.info.sender;
    let mut token = retrieve!(tokens[token_id.clone()])?;
    token.check_owner(&ctx)?;
    token.approve(&ctx, spender.clone(), expires)?;
    persist!(tokens[token_id.clone()] = token)?;
    emit!("approve", { owner, spender, token_id });
    Ok(())
  }

  pub fn approve_all(ctx: &mut ExecuteContext, operator: Addr, _expires: Option<Expiration>) -> Result<(), Cw721Error> {
    let owner = &ctx.info.sender;
    persist!(operators[(owner.clone(), operator.clone())] = _expires.unwrap_or(Expiration::Never {}))?;
    emit!("approve.all", { owner, operator });
    Ok(())
  }

  pub fn revoke(ctx: &mut ExecuteContext, token_id: String, spender: Addr) -> Result<(), Cw721Error> {
    let owner = &ctx.info.sender;
    let mut token = retrieve!(tokens[token_id.clone()])?;
    token.check_owner(&ctx)?;
    token.revoke_approval(&ctx, spender.clone())?;
    persist!(tokens[token_id.clone()] = token)?;
    emit!("revoke", { owner, token_id, spender });
    Ok(())
  }

  pub fn revoke_all(ctx: &mut ExecuteContext, operator: Addr) -> Result<(), Cw721Error> {
    let owner = &ctx.info.sender;
    delete!(operators[(owner.clone(), operator.clone())]);
    emit!("revoke.all", { owner, operator });
    Ok(())
  }

  #[authority(minter)]
  fn mint(
    ctx: &mut ExecuteContext,
    recipient: Addr,
    token_id: String,
    token_uri: String,
  ) -> Result<(), Cw721Error> {
    let token = Token {
      owner: recipient.clone(),
      approvals: vec![],
      token_uri,
    };
    persist!(tokens[token_id.clone()] = token)?;
    upstate!(CollectionInfo: { total_supply: old.total_supply + 1 })?;
    emit!("mint", { recipient, token_id });
    Ok(())
  }

  pub fn burn(ctx: &mut ExecuteContext, token_id: String) -> Result<(), Cw721Error> {
    let mut token = retrieve!(tokens[token_id.clone()])?;
    token.check_owner(&ctx)?;
    token.owner = Addr::unchecked("");
    persist!(tokens[token_id.clone()] = token)?;
    upstate!(CollectionInfo: { total_supply: old.total_supply - 1 })?;
    emit!("burn", { token_id });
    Ok(())
  }
}

#[solarize(query)]
impl Cw721Contract {
  #[returns({
    owner: Addr,
    approvals: Vec<Approval>,
  })]
  pub fn owner_of(ctx: QueryContext, token_id: String) -> Result<_, Cw721Error> {
    let token = retrieve!(tokens[token_id.clone()])?;
    Ok(response! {
      owner: token.owner,
      approvals: token.approvals,
    })
  }

  #[returns({
    approval: Approval,
  })]
  pub fn approval(ctx: QueryContext, token_id: String, spender: Addr, include_expired: Option<bool>) -> Result<_, Cw721Error> {
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

  #[returns({
    approvals: Vec<Approval>,
  })]
  pub fn approvals(ctx: QueryContext, token_id: String, include_expired: Option<bool>) -> Result<_, Cw721Error> {
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
    Ok(response! {
      approvals: explicit.chain(implicit).collect(),
    })
  }

  #[returns({
    approval: Approval
  })]
  pub fn operator(ctx: QueryContext, owner: Addr, operator: Addr, include_expired: Option<bool>) -> Result<_, Cw721Error> {
    let include_expired = include_expired.unwrap_or(false);
    let expires = retrieve!(operators[(owner.clone(), operator.clone())])
      .map_err(|_| ContractError::NotFound)
      .and_then(|x| if !include_expired && x.is_expired(&ctx.env.block) { Err(ContractError::NotFound) } else { Ok(x) })?;
    Ok(response! {
      approval: Approval {
        spender: operator,
        expires,
      },
    })
  }

  #[returns({
    operators: Vec<Approval>,
    next: Option<Addr>,
  })]
  pub fn all_operators(
    ctx: QueryContext,
    owner: Addr,
    include_expired: Option<bool>,
    start_after: Option<Addr>,
    limit: Option<u32>,
  ) -> Result<_, Cw721Error> {
    let iter = enumerate!(operators[owner], start_after.., descending);
    let limit = limit.unwrap_or(100) as usize;
    let include_expired = include_expired.unwrap_or(false);

    let mut operators = iter
      .filter(|op|
        op.is_ok() && (include_expired || !op.as_ref().unwrap().1.is_expired(&ctx.env.block)))
      .take(limit + 1)
      .collect::<Result<Vec<_>, _>>()?;

    let next = if operators.len() == limit + 1 {
      Some(operators.pop().unwrap().0)
    } else {
      None
    };
    Ok(response! {
      operators: operators
        .into_iter()
        .map(|(spender, expires)| Approval {
          spender,
          expires,
        })
        .collect::<Vec<_>>(),
      next,
    })
  }

  #[returns({
    name: String,
    symbol: String,
    description: Option<String>,
    total_supply: u64,
    cap: Option<u64>,
  })]
  fn collection_info(ctx: QueryContext) -> Result<_, Cw721Error> {
    let info = retrieve!(CollectionInfo)?;
    Ok(response! {
      name: info.name,
      symbol: info.symbol,
      description: info.description,
      total_supply: info.total_supply,
      cap: info.cap,
    })
  }

  #[returns({
    owner: Addr,
    token_uri: String,
  })]
  fn nft_info(ctx: QueryContext, token_id: String) -> Result<_, Cw721Error> {
    let token = retrieve!(tokens[token_id.clone()])?;
    Ok(response! {
      owner: token.owner,
      token_uri: token.token_uri,
    })
  }
}
