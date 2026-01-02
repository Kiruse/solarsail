use serde::{Deserialize, Serialize};

pub use crate::context::{ExecuteContext, QueryContext};

// These types are runtime-specific. Once we add new runtimes, these will need to be feature-gated.
pub type ExecuteResponse = cosmwasm_std::Response;
pub type ExecuteError = cosmwasm_std::StdError;
pub type QueryResponse = cosmwasm_std::Binary;
pub type QueryError = cosmwasm_std::StdError;
pub type Addr = cosmwasm_std::Addr;
pub type Funds = Vec<cosmwasm_std::Coin>;

pub type ExecuteResult<E> = Result<ExecuteResponse, E>;
pub type QueryResult<E> = Result<QueryResponse, E>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Empty;

pub trait SolarsailContract {
  type InstantiateMsg: Serialize + Sized;
  type MigrateMsg: Serialize + Sized;
  type Execute: ExecuteMsg;
  type Query: QueryMsg;
  type Error: std::error::Error;

  fn instantiate(ctx: ExecuteContext, msg: Self::InstantiateMsg) -> ExecuteResult<Self::Error>;

  fn migrate(ctx: ExecuteContext, msg: Self::MigrateMsg) -> ExecuteResult<Self::Error>;
}

pub trait ExecuteMsg: Serialize + Sized {
  type Error: std::error::Error;

  /// Handle this execute message & return the result of the execution.
  fn handle(&self, ctx: &mut ExecuteContext) -> Result<(), Self::Error>;

  /// Execute this execute message on the given contract.
  fn execute(&self, ctx: &mut ExecuteContext, contract_addr: &Addr, funds: Funds) -> Result<(), ExecuteError> {
    ctx.invoke(contract_addr, self, funds)?;
    Ok(())
  }
}

pub trait QueryMsg: Serialize + Sized {
  type Error: std::error::Error;

  /// Handle this query message and return the result of the query.
  fn handle(&self, ctx: QueryContext) -> QueryResult<Self::Error>;

  /// Execute this query message on the given contract. The result can then be parsed with [`serde`].
  ///
  /// **Note:** This is considered a low-level function. [`solarsail_macros::contract`] will generate
  /// a specialized `QueryMsg` trait which exposes additional query methods to automatically
  /// deserialize the result.
  fn query(&self, ctx: &QueryContext, contract_addr: Addr) -> QueryResult<QueryError> {
    ctx.deps.querier.query_wasm_smart(contract_addr, self)
  }
}
