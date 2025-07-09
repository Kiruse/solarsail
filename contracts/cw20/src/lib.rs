pub type ContractResult<T> = std::result::Result<T, contract::ContractError>;

pub mod types;

#[solarsail::contract]
pub mod contract {
  use cosmwasm_std::{Addr, Uint128};
  use solarsail::*;

  use crate::types::*;

  state!({
    #[authority]
    minter: Option<Addr>,

    #[authority]
    marketing: Option<Addr>,
  });

  state!(TokenInfo, {
    name: String,
    symbol: String,
    decimals: u8,
    total_supply: Uint128,
    cap: Option<Uint128>,
  });

  state!(MarketingInfo, {
    project: Option<String>,
    description: Option<String>,
  });

  state!(LogoState, {
    logo: Option<Logo>,
  });

  state_map!(balances   : Addr => Uint128);
  state_map!(allowances : (Addr, Addr) => Allowance);

  error!(InsufficientBalance, "Insufficient balance");
  error!(Overflow(#[from] cosmwasm_std::OverflowError));

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

  pub fn migrate(ctx: ExecuteContext) -> ContractResult<ContractError> {
    Ok(())
  }

  #[contract(execute)]
  pub mod execute {
    use cosmwasm_std::Binary;

    use super::*;

    #[execute]
    fn transfer(ctx: ExecuteContext, recipient: Addr, amount: Uint128) -> ContractResult<ContractError> {
      let sender = &ctx.info.sender;

      let balance = retrieve!(balances[sender.clone()])?;
      solarsail::assert!(balance >= amount, ContractError::InsufficientBalance);
      persist!(balances[sender.clone()] = &(balance - amount))?;

      let balance = retrieve!(balances[recipient.clone()])?;
      persist!(balances[recipient.clone()] = &(balance + amount))?;
      emit!("transfer", { recipient, amount });
      Ok(())
    }

    #[execute]
    fn send(ctx: ExecuteContext, recipient: Addr, amount: Uint128, msg: Binary) -> ContractResult<ContractError> {
      let sender = &ctx.info.sender;
      let balance = retrieve!(balances[sender.clone()])?;

      solarsail::assert!(balance >= amount, ContractError::InsufficientBalance);
      persist!(balances[sender.clone()] = &(balance - amount))?;

      let recipient_balance = retrieve!(balances[recipient.clone()])?;
      persist!(balances[recipient.clone()] = &(recipient_balance + amount))?;

      invoke!(recipient, msg)?;
      emit!("send", { recipient, amount });
      Ok(())
    }

    #[execute]
    fn burn(ctx: ExecuteContext, amount: Uint128) -> ContractResult<ContractError> {
      let sender = &ctx.info.sender;
      let balance = retrieve!(balances[sender.clone()])?;
      solarsail::assert!(balance >= amount, ContractError::InsufficientBalance);
      persist!(balances[sender.clone()] = &(balance - amount))?;
      upstate!(TokenInfo: { total_supply: old.total_supply - amount })?;
      emit!("burn", { amount: amount.to_string() });
      Ok(())
    }

    #[execute]
    #[authority(minter)]
    fn mint(ctx: ExecuteContext, amount: Uint128, recipient: Addr) -> ContractResult<ContractError> {
      let balance = retrieve!(balances[recipient.clone()])?;
      persist!(balances[recipient.clone()] = &(balance + amount))?;
      upstate!(TOKEN_INFO: { total_supply: old.total_supply + amount })?;
      emit!("mint", { recipient, amount });
      Ok(())
    }

    #[execute]
    fn increase_allowance(ctx: ExecuteContext, spender: Addr, amount: Uint128, expiry: Option<Expiry>) -> ContractResult<ContractError> {
      let key = (ctx.info.sender.clone(), spender.clone());
      let allowance = retrieve!(allowances[key.clone()])?;
      persist!(allowances[key] = &(allowance.raise(amount)?.expire(expiry.clone())))?;
      emit!("allowance.increase", {
        spender,
        amount,
        expiry: expiry.unwrap_or(Expiry::Never),
      });
      Ok(())
    }

    #[execute]
    fn decrease_allowance(ctx: ExecuteContext, spender: Addr, amount: Uint128) -> ContractResult<ContractError> {
      let key = (ctx.info.sender.clone(), spender.clone());
      let allowance = retrieve!(allowances[key.clone()])?;
      persist!(allowances[key] = &(allowance.lower(amount)?))?;
      emit!("allowance.decrease", { spender, amount });
      Ok(())
    }

    #[execute]
    fn transfer_from(ctx: ExecuteContext, sender: Addr, recipient: Addr, amount: Uint128) -> ContractResult<ContractError> {
      let key = (ctx.info.sender.clone(), sender.clone());
      let allowance = retrieve!(allowances[key.clone()])?;
      persist!(allowances[key] = &(allowance.lower(amount)?))?;

      let balance = retrieve!(balances[sender.clone()])?;
      solarsail::assert!(balance >= amount, ContractError::InsufficientBalance);
      persist!(balances[sender.clone()] = &(balance.checked_sub(amount)?))?;

      let balance = retrieve!(balances[recipient.clone()])?;
      persist!(balances[recipient.clone()] = &(balance.checked_add(amount)?))?;
      emit!("transfer_from", { sender, recipient, amount });
      Ok(())
    }

    #[execute]
    fn send_from(ctx: ExecuteContext, sender: Addr, recipient: Addr, amount: Uint128, msg: Binary) -> ContractResult<ContractError> {
      let key = (ctx.info.sender.clone(), sender.clone());
      let allowance = retrieve!(allowances[key.clone()])?;
      persist!(allowances[key] = &(allowance.lower(amount)?))?;

      let balance = retrieve!(balances[sender.clone()])?;
      solarsail::assert!(balance >= amount, ContractError::InsufficientBalance);
      persist!(balances[sender.clone()] = &(balance - amount))?;

      let balance = retrieve!(balances[recipient.clone()])?;
      persist!(balances[recipient.clone()] = &(balance.checked_add(amount)?))?;

      invoke!(recipient, msg)?;
      emit!("send_from", { sender, recipient, amount });
      Ok(())
    }

    #[execute]
    // no need for #[authority(minter)] here because the `execute_transfer_authority` already enforces it
    fn update_minter(ctx: ExecuteContext, minter: Option<String>) -> ContractResult<ContractError> {
      execute_transfer_authority(ctx, TransferAuthority::Minter(minter.clone()))?;
      match minter {
        Some(minter) => emit!("authority.transfer", { which: "minter", minter }),
        None => emit!("authority.renounce", { which: "minter" }),
      }
      Ok(())
    }

    #[execute]
    // no need for #[authority(marketing)] here because the `execute_transfer_authority` already enforces it
    fn update_marketing(ctx: ExecuteContext, marketing: Option<String>) -> ContractResult<ContractError> {
      execute_transfer_authority(ctx, TransferAuthority::Marketing(marketing.clone()))?;
      match marketing {
        Some(marketing) => emit!("authority.transfer", { which: "marketing", marketing }),
        None => emit!("authority.renounce", { which: "marketing" }),
      }
      Ok(())
    }

    #[execute]
    #[authority(marketing)]
    fn upload_logo(ctx: ExecuteContext, #[msg] logo: Logo) -> ContractResult<ContractError> {
      persist!(LogoState {
        logo: Some(logo),
      })?;
      emit!("logo.update");
      Ok(())
    }
  }

  #[contract(query)]
  pub mod query {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::Binary;

    use super::*;

    #[cw_serde]
    pub struct BalanceResponse {
      pub balance: Uint128,
    }

    #[query]
    fn balance(ctx: QueryContext, address: Addr) -> Result<BalanceResponse, ContractError> {
      Ok(BalanceResponse {
        balance: retrieve!(balances[address])?,
      })
    }

    #[cw_serde]
    pub struct TokenInfoResponse {
      pub name: String,
      pub symbol: String,
      pub decimals: u8,
      pub total_supply: Uint128,
    }

    impl From<TokenInfo> for TokenInfoResponse {
      fn from(info: TokenInfo) -> Self {
        Self {
          name: info.name,
          symbol: info.symbol,
          decimals: info.decimals,
          total_supply: info.total_supply,
        }
      }
    }

    #[query]
    fn token_info(ctx: QueryContext) -> Result<TokenInfoResponse, ContractError> {
      Ok(retrieve!(TokenInfo)?.into())
    }

    #[cw_serde]
    pub struct AllowanceResponse {
      pub allowance: Uint128,
      pub expires: Expiry,
    }

    impl From<Allowance> for AllowanceResponse {
      fn from(allowance: Allowance) -> Self {
        Self {
          allowance: allowance.amount,
          expires: allowance.expiry.unwrap_or(Expiry::Never),
        }
      }
    }

    #[query]
    fn allowance(ctx: QueryContext, owner: Addr, spender: Addr) -> Result<AllowanceResponse, ContractError> {
      Ok(retrieve!(allowances[(owner, spender)])?.into())
    }

    #[cw_serde]
    pub struct MinterResponse {
      pub minter: String,
      pub cap: Option<Uint128>,
    }

    #[query]
    fn minter(ctx: QueryContext) -> Result<MinterResponse, ContractError> {
      let cap = retrieve!(TokenInfo)?.cap;
      Ok(MinterResponse {
        minter: query_authority(ctx, Authority::Minter)?
          .map(|m| m.to_string())
          .unwrap_or("".to_string()),
        cap,
      })
    }

    #[cw_serde]
    pub struct MarketingInfoResponse {
      /// A URL pointing to the project behind this token
      pub project: Option<String>,
      pub description: Option<String>,
      pub logo: Option<LogoInfo>,
      /// Address (if any) who can update this marketing info
      pub marketing: Option<Addr>,
    }

    #[query]
    fn marketing_info(ctx: QueryContext) -> Result<MarketingInfoResponse, ContractError> {
      let info = retrieve!(MarketingInfo)?;
      Ok(MarketingInfoResponse {
        project: info.project,
        description: info.description,
        logo: retrieve!(LogoState)?.logo.map(|l| l.into()),
        marketing: query_authority(ctx, Authority::Marketing)?,
      })
    }

    #[cw_serde]
    pub struct DownloadLogoResponse {
      pub mime_type: String,
      pub data: Binary,
    }

    #[query]
    fn download_logo(ctx: QueryContext) -> Result<DownloadLogoResponse, ContractError> {
      let logo = retrieve!(LogoState)?.logo;
      match logo {
        Some(Logo::Url(_)) =>
          return Err(ContractError::generic("Logo is a URL, cannot download. Query URL with `marketing_info` instead.")),
        Some(Logo::Embedded(EmbeddedLogo::Svg(data))) => Ok(DownloadLogoResponse {
          mime_type: "image/svg+xml".to_string(),
          data,
        }),
        Some(Logo::Embedded(EmbeddedLogo::Png(data))) => Ok(DownloadLogoResponse {
          mime_type: "image/png".to_string(),
          data,
        }),
        None => Err(ContractError::generic("Token has no logo.")),
      }
    }

    #[cw_serde]
    pub struct AllAllowancesResponse {
      pub allowances: Vec<AllowanceResponse>,
      pub next: Option<Addr>,
    }

    #[query]
    fn all_allowances(ctx: QueryContext, owner: Addr, start_after: Option<Addr>, limit: Option<u32>) -> Result<AllAllowancesResponse, ContractError> {
      let iter = enumerate!(allowances[owner], start_after.., descending);
      let limit = limit.unwrap_or(100) as usize;

      let mut allowances = iter
        .take(limit + 1)
        .collect::<Result<Vec<_>, _>>()?;

      let next = if allowances.len() == limit + 1 {
        Some(allowances.pop().unwrap().0)
      } else {
        None
      };

      Ok(AllAllowancesResponse {
        allowances: allowances.into_iter().map(|(_, allowance)| allowance.into()).collect(),
        next,
      })
    }

    #[cw_serde]
    pub struct AllAccountsResponse {
      pub accounts: Vec<Addr>,
      pub next: Option<Addr>,
    }

    #[query]
    fn all_accounts(ctx: QueryContext, start_after: Option<Addr>, limit: Option<u32>) -> Result<AllAccountsResponse, ContractError> {
      let iter = enumerate!(balances, start_after.., descending);
      let limit = limit.unwrap_or(100) as usize;

      let mut accounts = iter
        .take(limit + 1)
        .map(|res| res.map(|(addr, _)| addr))
        .collect::<Result<Vec<_>, _>>()?;

      let next = if accounts.len() == limit + 1 {
        Some(accounts.pop().unwrap())
      } else {
        None
      };

      Ok(AllAccountsResponse {
        accounts,
        next,
      })
    }
  }
}
