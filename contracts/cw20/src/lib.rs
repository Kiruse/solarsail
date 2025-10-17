use cosmwasm_std::{Addr, Binary, Uint128, to_json_binary};
use solarsail::{scaffold::ExecuteResult, *};

pub mod types;
use types::*;

contract! {
  name! Cw20;

  authority! [minter, marketing];

  state! TokenInfo {
    name: String,
    symbol: String,
    decimals: u8,
    total_supply: Uint128,
    cap: Option<Uint128>,
  }

  state! MarketingInfo {
    project: Option<String>,
    description: Option<String>,
    logo: Option<Logo>,
  }

  state! Balances   : Addr => Uint128;
  state! Allowances : (Addr, Addr) => Allowance;

  error! InsufficientBalance : "Insufficient balance";
  error! Overflow(#[from] cosmwasm_std::OverflowError) : "Overflow: {0}";
}

#[solarize]
fn instantiate(
  mut ctx: ExecuteContext,
  name: String,
  symbol: String,
  decimals: Option<u8>,
  cap: Option<Uint128>,
) -> ExecuteResult<Cw20Error> {
  persist!(TokenInfo {
    name,
    symbol,
    decimals: decimals.unwrap_or(6u8),
    total_supply: Uint128::zero(),
    cap,
  })?;

  Ok(())
}

#[solarize]
fn migrate(ctx: ExecuteContext) -> ExecuteResult<Cw20Error> {
  Ok(())
}

#[solarize(execute)]
impl Cw20Contract {
  pub fn transfer(ctx: &mut ExecuteContext, recipient: Addr, amount: Uint128) -> Result<(), Cw20Error> {
    let sender = &ctx.info.sender;

    let balance = retrieve!(balances[sender.clone()])?;
    solarsail::assert!(balance >= amount, Cw20Error::InsufficientBalance);
    persist!(balances[sender.clone()] = &(balance - amount))?;

    let balance = retrieve!(balances[recipient.clone()])?;
    persist!(balances[recipient.clone()] = &(balance + amount))?;
    emit!("transfer", { recipient, amount });
    Ok(())
  }

  pub fn send(ctx: &mut ExecuteContext, recipient: Addr, amount: Uint128, msg: Binary) -> Result<(), Cw20Error> {
    let sender = &ctx.info.sender;
    let balance = retrieve!(balances[sender.clone()])?;

    solarsail::assert!(balance >= amount, Cw20Error::InsufficientBalance);
    persist!(balances[sender.clone()] = &(balance - amount))?;

    let recipient_balance = retrieve!(balances[recipient.clone()])?;
    persist!(balances[recipient.clone()] = &(recipient_balance + amount))?;

    let msg = to_json_binary(&Cw20ReceiverExecuteMsg::Receive(Cw20ReceiveMsg {
      sender: ctx.info.sender.clone(),
      amount: amount.clone(),
      msg,
    }))?;
    invoke!(recipient, msg);
    emit!("send", { recipient, amount });
    Ok(())
  }

  pub fn burn(ctx: &mut ExecuteContext, amount: Uint128) -> Result<(), Cw20Error> {
    let sender = &ctx.info.sender;
    let balance = retrieve!(balances[sender.clone()])?;
    solarsail::assert!(balance >= amount, Cw20Error::InsufficientBalance);
    persist!(balances[sender.clone()] = &(balance - amount))?;
    upstate!(TokenInfo: { total_supply: old.total_supply - amount })?;
    emit!("burn", { amount });
    Ok(())
  }

  #[authority(minter)]
  fn mint(ctx: &mut ExecuteContext, amount: Uint128, recipient: Addr) -> Result<(), Cw20Error> {
    let balance = retrieve!(balances[recipient.clone()])?;
    persist!(balances[recipient.clone()] = &(balance + amount))?;
    upstate!(TOKEN_INFO: { total_supply: old.total_supply + amount })?;
    emit!("mint", { recipient, amount });
    Ok(())
  }

  pub fn increase_allowance(ctx: &mut ExecuteContext, spender: Addr, amount: Uint128, expiry: Option<Expiry>) -> Result<(), Cw20Error> {
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

  pub fn decrease_allowance(ctx: &mut ExecuteContext, spender: Addr, amount: Uint128) -> Result<(), Cw20Error> {
    let key = (ctx.info.sender.clone(), spender.clone());
    let allowance = retrieve!(allowances[key.clone()])?;
    persist!(allowances[key] = &(allowance.lower(amount)?))?;
    emit!("allowance.decrease", { spender, amount });
    Ok(())
  }

  pub fn transfer_from(ctx: &mut ExecuteContext, sender: Addr, recipient: Addr, amount: Uint128) -> Result<(), Cw20Error> {
    let key = (ctx.info.sender.clone(), sender.clone());
    let allowance = retrieve!(allowances[key.clone()])?;
    persist!(allowances[key] = &(allowance.lower(amount)?))?;

    let balance = retrieve!(balances[sender.clone()])?;
    solarsail::assert!(balance >= amount, Cw20Error::InsufficientBalance);
    persist!(balances[sender.clone()] = &(balance.checked_sub(amount)?))?;

    let balance = retrieve!(balances[recipient.clone()])?;
    persist!(balances[recipient.clone()] = &(balance.checked_add(amount)?))?;
    emit!("transfer_from", { sender, recipient, amount });
    Ok(())
  }

  pub fn send_from(ctx: &mut ExecuteContext, sender: Addr, recipient: Addr, amount: Uint128, msg: Binary) -> Result<(), Cw20Error> {
    let key = (ctx.info.sender.clone(), sender.clone());
    let allowance = retrieve!(allowances[key.clone()])?;
    persist!(allowances[key] = &(allowance.lower(amount)?))?;

    let balance = retrieve!(balances[sender.clone()])?;
    solarsail::assert!(balance >= amount, Cw20Error::InsufficientBalance);
    persist!(balances[sender.clone()] = &(balance - amount))?;

    let balance = retrieve!(balances[recipient.clone()])?;
    persist!(balances[recipient.clone()] = &(balance.checked_add(amount)?))?;

    invoke!(recipient, msg);
    emit!("send_from", { sender, recipient, amount });
    Ok(())
  }

  // Instead of `update_minter` and `update_marketing`, we have `authority` as a central point for
  // all authority-related operations.
  fn authority(ctx: &mut ExecuteContext, #[msg] op: AuthorityOperation) -> Result<(), Cw20Error> {
    op.handle(ctx)?;
    Ok(())
  }

  #[authority(marketing)]
  fn upload_logo(ctx: &mut ExecuteContext, #[msg] logo: Logo) -> Result<(), Cw20Error> {
    upstate!(MarketingInfo: {
      logo: Some(logo),
    })?;
    emit!("logo.update");
    Ok(())
  }
}

#[solarize(query)]
impl Cw20Contract {
  #[returns({
    balance: Uint128,
  })]
  pub fn balance(ctx: QueryContext, address: Addr) -> Result<_, Cw20Error> {
    Ok(response! {
      balance: retrieve!(balances[address])?,
    })
  }

  #[returns({
    name: String,
    symbol: String,
    decimals: u8,
    total_supply: Uint128,
  })]
  pub fn token_info(ctx: QueryContext) -> Result<_, Cw20Error> {
    let info = retrieve!(TokenInfo)?;
    Ok(response! {
      name: info.name,
      symbol: info.symbol,
      decimals: info.decimals,
      total_supply: info.total_supply,
    })
  }

  #[returns({
    allowance: Uint128,
    expires: Expiry,
  })]
  pub fn allowance(ctx: QueryContext, owner: Addr, spender: Addr) -> Result<_, Cw20Error> {
    let key = (owner.clone(), spender.clone());
    Ok(response! {
      allowance: retrieve!(allowances[key.clone()])?.amount,
      expires: retrieve!(allowances[key])?.expiry.unwrap_or(Expiry::Never),
    })
  }

  #[returns({
    minter: Addr,
    cap: Option<Uint128>,
  })]
  pub fn minter(ctx: QueryContext) -> Result<_, Cw20Error> {
    let cap = retrieve!(TokenInfo)?.cap;
    Ok(response! {
      minter: Authority::Minter.get(ctx.deps.storage)?,
      cap,
    })
  }

  #[returns({
    project: String,
    description: String,
    logo: Option<LogoInfo>,
    marketing: Option<Addr>,
  })]
  pub fn marketing_info(ctx: QueryContext) -> Result<_, Cw20Error> {
    let info = retrieve!(MarketingInfo)?;
    Ok(response! {
      project: info.project.unwrap_or_default(),
      description: info.description.unwrap_or_default(),
      logo: info.logo.map(|l| l.into()),
      marketing: Authority::Marketing.get_maybe(ctx.deps.storage)?,
    })
  }

  #[returns({
    mime_type: String,
    data: Binary,
  })]
  pub fn download_logo(ctx: QueryContext) -> Result<_, Cw20Error> {
    let logo = retrieve!(MarketingInfo)?.logo;
    match logo {
      Some(Logo::Url(_)) =>
        return Err(Cw20Error::generic("Logo is a URL, cannot download. Query URL with `marketing_info` instead.")),
      Some(Logo::Embedded(EmbeddedLogo::Svg(data))) => Ok(response! {
        mime_type: "image/svg+xml".to_string(),
        data,
      }),
      Some(Logo::Embedded(EmbeddedLogo::Png(data))) => Ok(response! {
        mime_type: "image/png".to_string(),
        data,
      }),
      None => Err(Cw20Error::generic("Token has no logo.")),
    }
  }

  #[returns({
    allowances: Vec<Allowance>,
    next: Option<Addr>,
  })]
  pub fn all_allowances(ctx: QueryContext, owner: Addr, start_after: Option<Addr>, limit: Option<u32>) -> Result<_, Cw20Error> {
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

    Ok(response! {
      allowances: allowances
        .into_iter()
        .map(|(_, allowance)| allowance)
        .collect::<Vec<_>>(),
      next,
    })
  }

  #[returns({
    accounts: Vec<Addr>,
    next: Option<Addr>,
  })]
  pub fn all_accounts(ctx: QueryContext, start_after: Option<Addr>, limit: Option<u32>) -> Result<_, Cw20Error> {
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

    Ok(response! {
      accounts,
      next,
    })
  }
}
