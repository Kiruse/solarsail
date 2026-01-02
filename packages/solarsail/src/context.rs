use cosmwasm_std::{CosmosMsg, Deps, DepsMut, Env, Event, MessageInfo, SubMsg, WasmMsg, to_json_binary};
use serde::Serialize;

use crate::scaffold::{Addr, ExecuteError, Funds};

pub(crate) trait ExecuteContextDetails<'a> {
  fn deps_mut(&'_ mut self) -> &mut DepsMut<'a>;
  fn deps(&'_ self) -> Deps<'_>;
  fn env(&self) -> &Env;
  fn info(&self) -> &MessageInfo;
  fn submsgs(&self) -> &Vec<SubMsg>;
  fn events(&self) -> &Vec<Event>;
}

pub struct ExecuteContext<'a> {
  deps: DepsMut<'a>,
  env: Env,
  info: MessageInfo,
  submsgs: Vec<SubMsg>,
  events: Vec<Event>,
}

impl<'a> ExecuteContextDetails<'a> for ExecuteContext<'a> {
  fn deps_mut(&'_ mut self) -> &mut DepsMut<'a> {
    &mut self.deps
  }
  fn deps(&'_ self) -> Deps<'_> {
    self.deps.as_ref()
  }
  fn env(&self) -> &Env {
    &self.env
  }
  fn info(&self) -> &MessageInfo {
    &self.info
  }
  fn submsgs(&self) -> &Vec<SubMsg> {
    &self.submsgs
  }
  fn events(&self) -> &Vec<Event> {
    &self.events
  }
}

impl<'a> ExecuteContext<'a> {
  pub fn new(deps: DepsMut<'a>, env: Env, info: MessageInfo) -> Self {
    Self { deps, env, info, submsgs: vec![], events: vec![] }
  }

  /// Invoke a message on another smart contract.
  pub fn invoke(&mut self, contract_addr: &Addr, msg: impl Serialize + Sized, funds: Funds) -> Result<(), ExecuteError> {
    let msg = to_json_binary(&msg)?;
    self.submsgs.push(SubMsg::new(WasmMsg::Execute {
      contract_addr: contract_addr.to_string(),
      msg,
      funds,
    }));
    Ok(())
  }

  /// Call a chain-specific message or function.
  #[cfg(feature = "cosmwasm_2_0")]
  pub fn call_native(&mut self, msg: impl Into<CosmosMsg>) -> Result<(), ExecuteError> {
    self.submsgs.push(SubMsg::new(msg.into()));
    Ok(())
  }

  pub fn emit(&mut self, event: Event) {
    self.events.push(event);
  }
}

pub(crate) trait QueryContextDetails<'a> {
  fn deps(&self) -> &Deps<'a>;
  fn env(&self) -> &Env;
}

pub struct QueryContext<'a> {
  deps: Deps<'a>,
  env: Env,
}

impl<'a> QueryContextDetails<'a> for QueryContext<'a> {
  fn deps(&self) -> &Deps<'a> {
    &self.deps
  }
  fn env(&self) -> &Env {
    &self.env
  }
}

impl<'a> QueryContext<'a> {
  pub fn new(deps: Deps<'a>, env: Env) -> Self {
    Self { deps, env }
  }
}

impl<'a> From<&'a ExecuteContext<'a>> for QueryContext<'a> {
  fn from(ctx: &'a ExecuteContext<'a>) -> Self {
    Self::new(ctx.deps.as_ref(), ctx.env.clone())
  }
}
