use proc_macro2::TokenStream;
use quote::quote;

pub fn generate_error_struct(errors: &Vec<crate::parsers::ErrorDef>) -> TokenStream {
  quote! {
    #[derive(::thiserror::Error, Debug)]
    pub enum ContractError {
      #[error("{0}")]
      Std(#[from] ::cosmwasm_std::StdError),

      #[error("{0}")]
      Generic(String),

      #[error("Unauthorized")]
      Unauthorized,

      #(#errors,)*
    }

    impl ContractError {
      pub fn generic(msg: impl Into<String>) -> Self {
        Self::Generic(msg.into())
      }
    }
  }
}
