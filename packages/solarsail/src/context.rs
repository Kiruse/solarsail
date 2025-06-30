use cosmwasm_std::{Deps, DepsMut, Env, MessageInfo};

pub struct ExecuteContext<'a> {
  pub deps: DepsMut<'a>,
  pub env: Env,
  pub info: MessageInfo,
}

impl<'a> ExecuteContext<'a> {
  pub fn new(deps: DepsMut<'a>, env: Env, info: MessageInfo) -> Self {
    Self { deps, env, info }
  }
}

pub struct QueryContext<'a> {
  pub deps: Deps<'a>,
  pub env: Env,
}

impl<'a> QueryContext<'a> {
  pub fn new(deps: Deps<'a>, env: Env) -> Self {
    Self { deps, env }
  }
}
