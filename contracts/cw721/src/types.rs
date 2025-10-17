use cosmwasm_std::{Addr, Binary};
use cw_utils::Expiration;
use solarsail::{ExecuteContext, authority::AuthorityError, solarize};

use crate::Cw721Error;

#[solarize]
pub struct Token {
  /// Owner of the NFT. When None, the NFT is burnt.
  pub owner: Addr,
  /// Approvals the NFT's owner has granted to other addresses. These addresses can `transfer` or
  /// `send` the NFT.
  pub approvals: Vec<Approval>,
  /// Official external URI for this NFT, whatever it may contain. Typically contains JSON metadata
  /// conformant to the [OpenSea NFT metadata standard](https://docs.opensea.io/docs/metadata-standards).
  /// May be an empty string.
  pub token_uri: String,
}

impl Token {
  pub fn check_owner(&self, ctx: &ExecuteContext) -> Result<(), Cw721Error> {
    if self.owner != ctx.info.sender {
      Err(AuthorityError::Unauthorized.into())
    } else {
      Ok(())
    }
  }

  pub fn check_allowance(&self, ctx: &ExecuteContext) -> Result<(), Cw721Error> {
    if self.approvals.iter().any(|approval| approval.spender == ctx.info.sender && !approval.expires.is_expired(&ctx.env.block)) {
      Ok(())
    } else {
      Err(AuthorityError::Unauthorized.into())
    }
  }

  pub fn approve(&mut self, ctx: &ExecuteContext, spender: Addr, expires: Option<Expiration>) -> Result<(), Cw721Error> {
    self.revoke_approval(ctx, spender.clone())?;
    self.approvals.push(Approval {
      spender,
      expires: expires.unwrap_or(Expiration::Never {}),
    });
    Ok(())
  }

  pub fn revoke_approval(&mut self, ctx: &ExecuteContext, spender: Addr) -> Result<(), Cw721Error> {
    self.approvals.retain(|approval| approval.spender != spender && !approval.expires.is_expired(&ctx.env.block));
    Ok(())
  }
}

#[solarize]
pub enum TokenMetadata {
  External(String),
  Embedded(EmbeddedTokenMetadata),
}

#[solarize]
pub struct EmbeddedTokenMetadata {
  /// Optional official name of this Token.
  pub name: Option<String>,
  /// Optional description of this Token.
  pub description: Option<String>,
  /// Optional external URL of arbitrary associated data, e.g. a video or audio file.
  pub external_url: Option<String>,
  /// Optional external URL of the associated image.
  pub image: Option<String>,
  /// Optional external URL of the associated animation.
  pub animation_url: Option<String>,
  /// An optional background color to use in frontend themes.
  pub background_color: Option<String>,
  /// Optional attributes of this Token.
  pub attributes: Vec<TokenAttribute>,
}

#[solarize]
pub struct TokenAttribute {
  pub trait_type: String,
  pub value: AttributeValue,
  /// Optional display type for this attribute.
  pub display_type: Option<String>,
}

#[solarize]
#[serde(untagged)]
pub enum AttributeValue {
  String(String),
  Number(f64),
  Boolean(bool),
}

#[solarize]
pub struct Approval {
  pub spender: Addr,
  pub expires: Expiration,
}

#[solarize]
pub enum Cw721ReceiverExecuteMsg {
  ReceiveNft(Cw721ReceiveMsg),
}

#[solarize]
pub struct Cw721ReceiveMsg {
  pub sender: Addr,
  pub token_id: String,
  pub msg: Binary,
}
