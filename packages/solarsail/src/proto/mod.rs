use prost::Message;

#[derive(Message)]
pub struct Any {
  #[prost(string, tag = "1")]
  pub type_url: String,
  #[prost(bytes, tag = "2")]
  pub value: Vec<u8>,
}

#[cfg(feature = "cosmwasm")]
impl From<Any> for cosmwasm_std::AnyMsg {
  fn from(any: Any) -> Self {
    cosmwasm_std::AnyMsg {
      type_url: any.type_url,
      value: any.value.into(),
    }
  }
}

#[cfg(feature = "cosmwasm")]
impl From<cosmwasm_std::AnyMsg> for Any {
  fn from(any: cosmwasm_std::AnyMsg) -> Self {
    Any {
      type_url: any.type_url,
      value: any.value.into(),
    }
  }
}

pub trait ProtobufAny where Self: Message + Default + Sized {
  const TYPE_URL: &'static str;

  fn to_any(&self) -> Any {
    let mut value: Vec<u8> = vec![];
    self.encode(&mut value).unwrap();
    Any {
      type_url: Self::TYPE_URL.to_string(),
      value,
    }
  }

  fn from_any(any: Any) -> Result<Self, prost::DecodeError> {
    if any.type_url != Self::TYPE_URL {
      return Err(prost::DecodeError::new("Invalid type URL"));
    }
    Self::decode(&mut any.value.as_slice())
  }
}

#[cfg(feature = "cosmwasm")]
pub mod cosmos {
  use prost::Message;

  #[derive(Message)]
  pub struct Coin {
    #[prost(string, tag = "1")]
    pub denom: String,
    #[prost(string, tag = "2")]
    pub amount: String,
  }

  #[derive(Message)]
  pub struct CoinMetadata {
    #[prost(optional, string, tag = "1")]
    pub description: Option<String>,
    #[prost(repeated, message, tag = "2")]
    pub denom_units: Vec<DenomUnit>,
    #[prost(string, tag = "3")]
    pub base: String,
    #[prost(optional, string, tag = "4")]
    pub display: Option<String>,
    #[prost(string, tag = "5")]
    pub name: String,
    #[prost(string, tag = "6")]
    pub symbol: String,
    #[prost(optional, string, tag = "7")]
    pub uri: Option<String>,
    #[prost(optional, string, tag = "8")]
    pub uri_hash: Option<String>,
  }

  #[derive(Message)]
  pub struct DenomUnit {
    #[prost(string, tag = "1")]
    pub denom: String,
    #[prost(uint32, tag = "2")]
    pub exponent: u32,
  }
}

#[cfg(feature = "cosmwasm")]
pub mod osmosis {
  use super::cosmos;

  pub mod tokenfactory {
    use prost::Message;

    use super::super::ProtobufAny;
    use super::cosmos::{Coin, CoinMetadata};

    #[derive(Message)]
    pub struct MsgCreateDenom {
      #[prost(string, tag = "1")]
      pub sender: String,
      #[prost(string, tag = "2")]
      pub subdenom: String,
    }

    impl ProtobufAny for MsgCreateDenom {
      const TYPE_URL: &'static str = "/osmosis.tokenfactory.v1beta1.MsgCreateDenom";
    }

    #[derive(Message)]
    pub struct MsgMint {
      #[prost(required, string, tag = "1")]
      pub sender: String,
      #[prost(required, message, tag = "2")]
      pub amount: Coin,
    }

    impl ProtobufAny for MsgMint {
      const TYPE_URL: &'static str = "/osmosis.tokenfactory.v1beta1.MsgMint";
    }

    #[derive(Message)]
    pub struct MsgBurn {
      #[prost(required, string, tag = "1")]
      pub sender: String,
      #[prost(required, message, tag = "2")]
      pub amount: Coin,
    }

    impl ProtobufAny for MsgBurn {
      const TYPE_URL: &'static str = "/osmosis.tokenfactory.v1beta1.MsgBurn";
    }

    #[derive(Message)]
    pub struct MsgChangeAdmin {
      #[prost(required, string, tag = "1")]
      pub sender: String,
      #[prost(required, string, tag = "2")]
      pub denom: String,
      #[prost(required, string, tag = "3")]
      pub new_admin: String,
    }

    impl ProtobufAny for MsgChangeAdmin {
      const TYPE_URL: &'static str = "/osmosis.tokenfactory.v1beta1.MsgChangeAdmin";
    }

    #[derive(Message)]
    pub struct MsgSetDenomMetadata {
      #[prost(required, string, tag = "1")]
      pub sender: String,
      #[prost(required, string, tag = "2")]
      pub denom: String,
      #[prost(required, message, tag = "3")]
      pub metadata: CoinMetadata,
    }

    impl ProtobufAny for MsgSetDenomMetadata {
      const TYPE_URL: &'static str = "/osmosis.tokenfactory.v1beta1.MsgSetDenomMetadata";
    }
  }
}
