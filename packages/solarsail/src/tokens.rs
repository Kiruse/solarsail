use solarsail_macros::solarize;

use crate::{Binary, ExecuteContext, Uint128, Uint256, scaffold::Addr};
use crate::context::ExecuteContextDetails;

#[cfg(feature = "cosmwasm")]
use crate::proto::osmosis::tokenfactory::{MsgCreateDenom, MsgMint, MsgBurn, MsgChangeAdmin, MsgSetDenomMetadata};
use crate::proto::ProtobufAny;

use crate as solarsail;

#[solarize]
pub struct TokenMetadata {
  /// Full name of the token.
  pub name: String,
  /// Symbol aka. ticker of the token, used for display only.
  pub symbol: String,
  /// Unique identifier & name of the smallest unit of the token. In Cosmos, this is typically
  /// "u" (micro) + the symbol in lowercase. By example of ATOM, the unit is uatom. By example of
  /// ETH, the unit is wei.
  pub denom: String,
  /// Description of the token. Not all chains support this.
  pub description: Option<String>,
  /// Image of the token. Not all chains support this.
  pub image: Option<String>,
  /// Number of decimals in the unit denomination, used to represent the token in the UI. Not all chains support this.
  pub decimals: u8,
}

impl From<TokenMetadata> for crate::proto::cosmos::CoinMetadata {
  fn from(metadata: TokenMetadata) -> Self {
    crate::proto::cosmos::CoinMetadata {
      name: metadata.name,
      symbol: metadata.symbol.clone(),
      description: metadata.description,
      denom_units: vec![
        crate::proto::cosmos::DenomUnit {
          denom: metadata.denom.clone(),
          exponent: 0,
        },
        crate::proto::cosmos::DenomUnit {
          denom: metadata.symbol.clone(),
          exponent: metadata.decimals as u32,
        },
      ],
      base: metadata.denom,
      display: Some(metadata.symbol),
      uri: metadata.image,
      uri_hash: None,
    }
  }
}

/// Represents a fungible token with a unique identifier.
pub enum FungibleToken {
  /// Represents a chain-native token.
  Native(String),
  /// Represents a smart contract-based token.
  Token(String),
}

#[cfg(feature = "cosmwasm")]
pub type FungibleTokenError = cosmwasm_std::StdError;

impl FungibleToken {
  /// Send tokens to a recipient.
  ///
  /// **Note:** Burning tokens is non-standard. Often, the native token does not directly support
  /// burning, so instead the common workaround is to transfer the tokens to a burn address.
  /// Typically, this address is the null address (all bytes are zero), as it has no valid private
  /// key in any contemporary cryptographic framework.
  pub fn transfer(&self, ctx: &mut ExecuteContext, recipient: &Addr, amount: Uint128) -> Result<(), FungibleTokenError> {
    match self {
      FungibleToken::Native(denom) => {
        ctx.call_native(cosmwasm_std::BankMsg::Send {
          to_address: recipient.to_string(),
          amount: vec![cosmwasm_std::coin(amount.u128(), denom)],
        })?;
        Ok(())
      }
      FungibleToken::Token(addr) => {
        let addr = ctx.deps().api.addr_validate(addr)?;
        ctx.invoke(
          &addr,
          &Cw20ExecuteMsg::Transfer {
            recipient: recipient.to_string(),
            amount,
          },
          vec![],
        )?;
        Ok(())
      }
    }
  }

  /// Send tokens to a recipient with a callback message. Note that this requires that the recipient
  /// is a smart contract or can otherwise process the callback message.
  pub fn send(&self, ctx: &mut ExecuteContext, recipient: &Addr, amount: Uint128, msg: Binary) -> Result<(), FungibleTokenError> {
    match self {
      FungibleToken::Native(denom) => {
        ctx.invoke(recipient, msg, vec![cosmwasm_std::coin(amount.u128(), denom)]);
        Ok(())
      }
      FungibleToken::Token(addr) => {
        let addr = ctx.deps().api.addr_validate(addr)?;
        ctx.invoke(
          &addr,
          &Cw20ExecuteMsg::Send {
            recipient: recipient.to_string(),
            amount,
            msg,
          },
          vec![],
        )?;
        Ok(())
      }
    }
  }
}

pub trait TokenFactory {
  /// Create a new token.
  fn create(&self, ctx: &mut ExecuteContext, metadata: TokenMetadata) -> Result<(), FungibleTokenError>;

  /// Update the metadata of a token. This may not be supported depending on the token factory
  /// and/or chain.
  fn update_metadata(&self, ctx: &mut ExecuteContext, metadata: TokenMetadata) -> Result<(), FungibleTokenError>;

  /// Mint tokens to a recipient. Only works if you have the appropriate permissions.
  fn mint(&self, ctx: &mut ExecuteContext, denom: String, amount: Uint128, recipient: Addr) -> Result<(), FungibleTokenError>;
}

/// The chain-specific standard token factory is designed to be easily usable on any compatible chain,
/// which is assumed to have one unique & centralized implementation. In Cosmos, this is the
/// TokenFactory chain module where applicable. On Solana, this is the SPL-Token program. All
/// [TokenFactory] implementations are expected to expose the same minimal interface, but they may
/// expose additional specialized functionality.
#[cfg(feature = "standard_token_factory")]
pub struct StandardTokenFactory;

// NOTE for the future:
// - Injective has its own TokenFactory implementation.
// - Neutron has a compatible TokenFactory implementation with custom message types.
// - Osmosis may have different type URLs for the same message types.
// - Solana has the SPL-Token program + Metaplex metadata program.
#[cfg(all(feature = "standard_token_factory", feature = "cosmwasm"))]
impl TokenFactory for StandardTokenFactory {
  fn create(&self, ctx: &mut ExecuteContext, metadata: TokenMetadata) -> Result<(), FungibleTokenError> {
    let sender = ctx.env().contract.address.to_string();

    ctx.call_native(MsgCreateDenom {
      sender: sender.clone(),
      subdenom: metadata.denom.clone(),
    }.to_any())?;

    ctx.call_native(MsgSetDenomMetadata {
      sender: sender.clone(),
      denom: metadata.denom.clone(),
      metadata: metadata.into(),
    }.to_any())?;

    Ok(())
  }

  fn update_metadata(&self, ctx: &mut ExecuteContext, metadata: TokenMetadata) -> Result<(), FungibleTokenError> {
    ctx.call_native(MsgSetDenomMetadata {
      sender: ctx.env().contract.address.to_string(),
      denom: metadata.denom.clone(),
      metadata: metadata.into(),
    }.to_any())?;
    Ok(())
  }

  fn mint(&self, ctx: &mut ExecuteContext, denom: String, amount: Uint128, recipient: Addr) -> Result<(), FungibleTokenError> {
    let sender = ctx.env().contract.address.to_string();

    ctx.call_native(MsgMint {
      sender,
      amount: crate::proto::cosmos::Coin {
        denom: denom.clone(),
        amount: amount.to_string(),
      },
    }.to_any())?;

    ctx.call_native(cosmwasm_std::BankMsg::Send {
      to_address: recipient.to_string(),
      amount: vec![cosmwasm_std::coin(amount.u128(), denom)],
    })?;

    Ok(())
  }
}

#[cfg(feature = "cosmwasm")]
#[solarize]
enum Cw20ExecuteMsg {
  Transfer {
    recipient: String,
    amount: Uint128,
  },
  Send {
    recipient: String,
    amount: Uint128,
    msg: Binary,
  },
  Burn {
    amount: Uint128,
  },
}
