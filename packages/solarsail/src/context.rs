use cosmwasm_std::{Deps, DepsMut, Env, Event, MessageInfo, SubMsg};

pub struct ExecuteContext<'a> {
  pub deps: DepsMut<'a>,
  pub env: Env,
  pub info: MessageInfo,
  pub submsgs: Vec<SubMsg>,
  pub events: Vec<Event>,
}

impl<'a> ExecuteContext<'a> {
  pub fn new(deps: DepsMut<'a>, env: Env, info: MessageInfo) -> Self {
    Self { deps, env, info, submsgs: vec![], events: vec![] }
  }

  pub fn invoke(&mut self, submsg: SubMsg) {
    self.submsgs.push(submsg);
  }

  pub fn emit(&mut self, event: Event) {
    self.events.push(event);
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

impl<'a> From<&'a ExecuteContext<'a>> for QueryContext<'a> {
  fn from(ctx: &'a ExecuteContext<'a>) -> Self {
    Self::new(ctx.deps.as_ref(), ctx.env.clone())
  }
}
