use cosmwasm_std::{Addr, Binary, Uint128};
use solarsail::solarize;

use crate::Cw20Error;

#[solarize]
pub struct Allowance {
  pub amount: Uint128,
  pub expiry: Option<Expiry>,
}

impl Allowance {
  pub fn raise(&self, amount: Uint128) -> Result<Self, Cw20Error> {
    Ok(Self {
      amount: self.amount.checked_add(amount)?,
      expiry: self.expiry.clone(),
    })
  }

  pub fn lower(&self, amount: Uint128) -> Result<Self, Cw20Error> {
    Ok(Self {
      amount: self.amount.checked_sub(amount)
        .map_err(|_| Cw20Error::InsufficientBalance)?,
      expiry: self.expiry.clone(),
    })
  }

  pub fn expire(&self, expiry: Option<Expiry>) -> Self {
    Self {
      amount: self.amount,
      expiry,
    }
  }
}

#[solarize]
pub enum Expiry {
  Never,
  AtTimestamp(cosmwasm_std::Uint64),
  AtBlockHeight(Uint128),
}

impl From<Expiry> for String {
  fn from(expiry: Expiry) -> Self {
    match expiry {
      Expiry::Never => "never".to_string(),
      Expiry::AtTimestamp(timestamp) => format!("at_timestamp({})", timestamp.to_string()),
      Expiry::AtBlockHeight(height) => format!("at_block_height({})", height.to_string()),
    }
  }
}

#[solarize]
pub enum Logo {
  Url(String),
  Embedded(EmbeddedLogo),
}

impl From<Logo> for LogoInfo {
  fn from(logo: Logo) -> Self {
    match logo {
      Logo::Url(url) => LogoInfo::Url(url),
      Logo::Embedded(_) => LogoInfo::Embedded,
    }
  }
}

#[solarize]
pub enum EmbeddedLogo {
  Svg(Binary),
  Png(Binary),
}

#[solarize]
pub enum LogoInfo {
  Url(String),
  Embedded,
}

#[solarize]
pub enum Cw20ReceiverExecuteMsg {
  Receive(Cw20ReceiveMsg),
}

#[solarize]
pub struct Cw20ReceiveMsg {
  pub sender: Addr,
  pub amount: Uint128,
  pub msg: Binary,
}
