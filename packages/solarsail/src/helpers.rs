use cosmwasm_std::{Response, StdError};

pub type ContractResult<E> = Result<Response, E>;
pub type StdContractResult = ContractResult<StdError>;
