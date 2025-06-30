#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use cosmwasm_std::StdError;
use thiserror::Error;
pub type ContractResult<T> = std::result::Result<T, ContractError>;
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("Unauthorized")]
    Unauthorized,
}
#[allow(unused_qualifications)]
#[automatically_derived]
impl std::error::Error for ContractError {
    fn source(&self) -> ::core::option::Option<&(dyn std::error::Error + 'static)> {
        use thiserror::__private::AsDynError as _;
        #[allow(deprecated)]
        match self {
            ContractError::Std { 0: source, .. } => {
                ::core::option::Option::Some(source.as_dyn_error())
            }
            ContractError::Unauthorized { .. } => ::core::option::Option::None,
        }
    }
}
#[allow(unused_qualifications)]
#[automatically_derived]
impl ::core::fmt::Display for ContractError {
    fn fmt(&self, __formatter: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        use thiserror::__private::AsDisplay as _;
        #[allow(unused_variables, deprecated, clippy::used_underscore_binding)]
        match self {
            ContractError::Std(_0) => {
                __formatter.write_fmt(format_args!("{0}", _0.as_display()))
            }
            ContractError::Unauthorized {} => __formatter.write_str("Unauthorized"),
        }
    }
}
#[allow(unused_qualifications)]
#[automatically_derived]
impl ::core::convert::From<StdError> for ContractError {
    #[allow(deprecated)]
    fn from(source: StdError) -> Self {
        ContractError::Std { 0: source }
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for ContractError {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            ContractError::Std(__self_0) => {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "Std", &__self_0)
            }
            ContractError::Unauthorized => {
                ::core::fmt::Formatter::write_str(f, "Unauthorized")
            }
        }
    }
}
pub mod contract {
    use cosmwasm_std::{Response, Uint128};
    use solarsail::*;
    use super::{ContractError, ContractResult};
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[serde(deny_unknown_fields, crate = "::cosmwasm_schema::serde")]
    #[schemars(crate = "::cosmwasm_schema::schemars")]
    pub struct State {
        pub minter: String,
        pub marketing: String,
    }
    #[doc(hidden)]
    #[allow(
        non_upper_case_globals,
        unused_attributes,
        unused_qualifications,
        clippy::absolute_paths,
    )]
    const _: () = {
        use ::cosmwasm_schema::serde as _serde;
        #[automatically_derived]
        impl ::cosmwasm_schema::serde::Serialize for State {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> ::cosmwasm_schema::serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: ::cosmwasm_schema::serde::Serializer,
            {
                let mut __serde_state = _serde::Serializer::serialize_struct(
                    __serializer,
                    "State",
                    false as usize + 1 + 1,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "minter",
                    &self.minter,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "marketing",
                    &self.marketing,
                )?;
                _serde::ser::SerializeStruct::end(__serde_state)
            }
        }
    };
    #[doc(hidden)]
    #[allow(
        non_upper_case_globals,
        unused_attributes,
        unused_qualifications,
        clippy::absolute_paths,
    )]
    const _: () = {
        use ::cosmwasm_schema::serde as _serde;
        #[automatically_derived]
        impl<'de> ::cosmwasm_schema::serde::Deserialize<'de> for State {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> ::cosmwasm_schema::serde::__private::Result<Self, __D::Error>
            where
                __D: ::cosmwasm_schema::serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                #[doc(hidden)]
                enum __Field {
                    __field0,
                    __field1,
                }
                #[doc(hidden)]
                struct __FieldVisitor;
                #[automatically_derived]
                impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type Value = __Field;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "field identifier",
                        )
                    }
                    fn visit_u64<__E>(
                        self,
                        __value: u64,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            0u64 => _serde::__private::Ok(__Field::__field0),
                            1u64 => _serde::__private::Ok(__Field::__field1),
                            _ => {
                                _serde::__private::Err(
                                    _serde::de::Error::invalid_value(
                                        _serde::de::Unexpected::Unsigned(__value),
                                        &"field index 0 <= i < 2",
                                    ),
                                )
                            }
                        }
                    }
                    fn visit_str<__E>(
                        self,
                        __value: &str,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            "minter" => _serde::__private::Ok(__Field::__field0),
                            "marketing" => _serde::__private::Ok(__Field::__field1),
                            _ => {
                                _serde::__private::Err(
                                    _serde::de::Error::unknown_field(__value, FIELDS),
                                )
                            }
                        }
                    }
                    fn visit_bytes<__E>(
                        self,
                        __value: &[u8],
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            b"minter" => _serde::__private::Ok(__Field::__field0),
                            b"marketing" => _serde::__private::Ok(__Field::__field1),
                            _ => {
                                let __value = &_serde::__private::from_utf8_lossy(__value);
                                _serde::__private::Err(
                                    _serde::de::Error::unknown_field(__value, FIELDS),
                                )
                            }
                        }
                    }
                }
                #[automatically_derived]
                impl<'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::__private::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        _serde::Deserializer::deserialize_identifier(
                            __deserializer,
                            __FieldVisitor,
                        )
                    }
                }
                #[doc(hidden)]
                struct __Visitor<'de> {
                    marker: _serde::__private::PhantomData<State>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                #[automatically_derived]
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = State;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "struct State",
                        )
                    }
                    #[inline]
                    fn visit_seq<__A>(
                        self,
                        mut __seq: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::SeqAccess<'de>,
                    {
                        let __field0 = match _serde::de::SeqAccess::next_element::<
                            String,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        0usize,
                                        &"struct State with 2 elements",
                                    ),
                                );
                            }
                        };
                        let __field1 = match _serde::de::SeqAccess::next_element::<
                            String,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        1usize,
                                        &"struct State with 2 elements",
                                    ),
                                );
                            }
                        };
                        _serde::__private::Ok(State {
                            minter: __field0,
                            marketing: __field1,
                        })
                    }
                    #[inline]
                    fn visit_map<__A>(
                        self,
                        mut __map: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::MapAccess<'de>,
                    {
                        let mut __field0: _serde::__private::Option<String> = _serde::__private::None;
                        let mut __field1: _serde::__private::Option<String> = _serde::__private::None;
                        while let _serde::__private::Some(__key) = _serde::de::MapAccess::next_key::<
                            __Field,
                        >(&mut __map)? {
                            match __key {
                                __Field::__field0 => {
                                    if _serde::__private::Option::is_some(&__field0) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field("minter"),
                                        );
                                    }
                                    __field0 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<String>(&mut __map)?,
                                    );
                                }
                                __Field::__field1 => {
                                    if _serde::__private::Option::is_some(&__field1) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field(
                                                "marketing",
                                            ),
                                        );
                                    }
                                    __field1 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<String>(&mut __map)?,
                                    );
                                }
                            }
                        }
                        let __field0 = match __field0 {
                            _serde::__private::Some(__field0) => __field0,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("minter")?
                            }
                        };
                        let __field1 = match __field1 {
                            _serde::__private::Some(__field1) => __field1,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("marketing")?
                            }
                        };
                        _serde::__private::Ok(State {
                            minter: __field0,
                            marketing: __field1,
                        })
                    }
                }
                #[doc(hidden)]
                const FIELDS: &'static [&'static str] = &["minter", "marketing"];
                _serde::Deserializer::deserialize_struct(
                    __deserializer,
                    "State",
                    FIELDS,
                    __Visitor {
                        marker: _serde::__private::PhantomData::<State>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    #[automatically_derived]
    #[allow(clippy::derive_partial_eq_without_eq)]
    impl ::core::clone::Clone for State {
        #[inline]
        fn clone(&self) -> State {
            State {
                minter: ::core::clone::Clone::clone(&self.minter),
                marketing: ::core::clone::Clone::clone(&self.marketing),
            }
        }
    }
    #[automatically_derived]
    #[allow(clippy::derive_partial_eq_without_eq)]
    impl ::core::fmt::Debug for State {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field2_finish(
                f,
                "State",
                "minter",
                &self.minter,
                "marketing",
                &&self.marketing,
            )
        }
    }
    #[automatically_derived]
    #[allow(clippy::derive_partial_eq_without_eq)]
    impl ::core::marker::StructuralPartialEq for State {}
    #[automatically_derived]
    #[allow(clippy::derive_partial_eq_without_eq)]
    impl ::core::cmp::PartialEq for State {
        #[inline]
        fn eq(&self, other: &State) -> bool {
            self.minter == other.minter && self.marketing == other.marketing
        }
    }
    const _: () = {
        use ::cosmwasm_schema::schemars as schemars;
        #[automatically_derived]
        #[allow(unused_braces)]
        impl schemars::JsonSchema for State {
            fn schema_name() -> std::string::String {
                "State".to_owned()
            }
            fn schema_id() -> std::borrow::Cow<'static, str> {
                std::borrow::Cow::Borrowed("solarsail_cw20::contract::State")
            }
            fn json_schema(
                generator: &mut schemars::gen::SchemaGenerator,
            ) -> schemars::schema::Schema {
                {
                    let mut schema_object = schemars::schema::SchemaObject {
                        instance_type: Some(
                            schemars::schema::InstanceType::Object.into(),
                        ),
                        ..Default::default()
                    };
                    let object_validation = schema_object.object();
                    object_validation.additional_properties = Some(
                        Box::new(false.into()),
                    );
                    {
                        schemars::_private::insert_object_property::<
                            String,
                        >(
                            object_validation,
                            "minter",
                            false,
                            false,
                            generator.subschema_for::<String>(),
                        );
                    }
                    {
                        schemars::_private::insert_object_property::<
                            String,
                        >(
                            object_validation,
                            "marketing",
                            false,
                            false,
                            generator.subschema_for::<String>(),
                        );
                    }
                    schemars::schema::Schema::Object(schema_object)
                }
            }
        }
    };
    pub const STATE: ::cw_storage_plus::Item<State> = ::cw_storage_plus::Item::new(
        "state",
    );
    struct Mint {
        pub amount: Uint128,
    }
    #[doc(hidden)]
    #[allow(
        non_upper_case_globals,
        unused_attributes,
        unused_qualifications,
        clippy::absolute_paths,
    )]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl _serde::Serialize for Mint {
            fn serialize<__S>(
                &self,
                __serializer: __S,
            ) -> _serde::__private::Result<__S::Ok, __S::Error>
            where
                __S: _serde::Serializer,
            {
                let mut __serde_state = _serde::Serializer::serialize_struct(
                    __serializer,
                    "Mint",
                    false as usize + 1,
                )?;
                _serde::ser::SerializeStruct::serialize_field(
                    &mut __serde_state,
                    "amount",
                    &self.amount,
                )?;
                _serde::ser::SerializeStruct::end(__serde_state)
            }
        }
    };
    #[doc(hidden)]
    #[allow(
        non_upper_case_globals,
        unused_attributes,
        unused_qualifications,
        clippy::absolute_paths,
    )]
    const _: () = {
        #[allow(unused_extern_crates, clippy::useless_attribute)]
        extern crate serde as _serde;
        #[automatically_derived]
        impl<'de> _serde::Deserialize<'de> for Mint {
            fn deserialize<__D>(
                __deserializer: __D,
            ) -> _serde::__private::Result<Self, __D::Error>
            where
                __D: _serde::Deserializer<'de>,
            {
                #[allow(non_camel_case_types)]
                #[doc(hidden)]
                enum __Field {
                    __field0,
                    __ignore,
                }
                #[doc(hidden)]
                struct __FieldVisitor;
                #[automatically_derived]
                impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                    type Value = __Field;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "field identifier",
                        )
                    }
                    fn visit_u64<__E>(
                        self,
                        __value: u64,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            0u64 => _serde::__private::Ok(__Field::__field0),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_str<__E>(
                        self,
                        __value: &str,
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            "amount" => _serde::__private::Ok(__Field::__field0),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                    fn visit_bytes<__E>(
                        self,
                        __value: &[u8],
                    ) -> _serde::__private::Result<Self::Value, __E>
                    where
                        __E: _serde::de::Error,
                    {
                        match __value {
                            b"amount" => _serde::__private::Ok(__Field::__field0),
                            _ => _serde::__private::Ok(__Field::__ignore),
                        }
                    }
                }
                #[automatically_derived]
                impl<'de> _serde::Deserialize<'de> for __Field {
                    #[inline]
                    fn deserialize<__D>(
                        __deserializer: __D,
                    ) -> _serde::__private::Result<Self, __D::Error>
                    where
                        __D: _serde::Deserializer<'de>,
                    {
                        _serde::Deserializer::deserialize_identifier(
                            __deserializer,
                            __FieldVisitor,
                        )
                    }
                }
                #[doc(hidden)]
                struct __Visitor<'de> {
                    marker: _serde::__private::PhantomData<Mint>,
                    lifetime: _serde::__private::PhantomData<&'de ()>,
                }
                #[automatically_derived]
                impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                    type Value = Mint;
                    fn expecting(
                        &self,
                        __formatter: &mut _serde::__private::Formatter,
                    ) -> _serde::__private::fmt::Result {
                        _serde::__private::Formatter::write_str(
                            __formatter,
                            "struct Mint",
                        )
                    }
                    #[inline]
                    fn visit_seq<__A>(
                        self,
                        mut __seq: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::SeqAccess<'de>,
                    {
                        let __field0 = match _serde::de::SeqAccess::next_element::<
                            Uint128,
                        >(&mut __seq)? {
                            _serde::__private::Some(__value) => __value,
                            _serde::__private::None => {
                                return _serde::__private::Err(
                                    _serde::de::Error::invalid_length(
                                        0usize,
                                        &"struct Mint with 1 element",
                                    ),
                                );
                            }
                        };
                        _serde::__private::Ok(Mint { amount: __field0 })
                    }
                    #[inline]
                    fn visit_map<__A>(
                        self,
                        mut __map: __A,
                    ) -> _serde::__private::Result<Self::Value, __A::Error>
                    where
                        __A: _serde::de::MapAccess<'de>,
                    {
                        let mut __field0: _serde::__private::Option<Uint128> = _serde::__private::None;
                        while let _serde::__private::Some(__key) = _serde::de::MapAccess::next_key::<
                            __Field,
                        >(&mut __map)? {
                            match __key {
                                __Field::__field0 => {
                                    if _serde::__private::Option::is_some(&__field0) {
                                        return _serde::__private::Err(
                                            <__A::Error as _serde::de::Error>::duplicate_field("amount"),
                                        );
                                    }
                                    __field0 = _serde::__private::Some(
                                        _serde::de::MapAccess::next_value::<Uint128>(&mut __map)?,
                                    );
                                }
                                _ => {
                                    let _ = _serde::de::MapAccess::next_value::<
                                        _serde::de::IgnoredAny,
                                    >(&mut __map)?;
                                }
                            }
                        }
                        let __field0 = match __field0 {
                            _serde::__private::Some(__field0) => __field0,
                            _serde::__private::None => {
                                _serde::__private::de::missing_field("amount")?
                            }
                        };
                        _serde::__private::Ok(Mint { amount: __field0 })
                    }
                }
                #[doc(hidden)]
                const FIELDS: &'static [&'static str] = &["amount"];
                _serde::Deserializer::deserialize_struct(
                    __deserializer,
                    "Mint",
                    FIELDS,
                    __Visitor {
                        marker: _serde::__private::PhantomData::<Mint>,
                        lifetime: _serde::__private::PhantomData,
                    },
                )
            }
        }
    };
    #[automatically_derived]
    impl ::core::clone::Clone for Mint {
        #[inline]
        fn clone(&self) -> Mint {
            Mint {
                amount: ::core::clone::Clone::clone(&self.amount),
            }
        }
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Mint {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "Mint",
                "amount",
                &&self.amount,
            )
        }
    }
    #[automatically_derived]
    impl ::core::marker::StructuralPartialEq for Mint {}
    #[automatically_derived]
    impl ::core::cmp::PartialEq for Mint {
        #[inline]
        fn eq(&self, other: &Mint) -> bool {
            self.amount == other.amount
        }
    }
    fn execute_mint(
        ctx: &::solarsail::ExecuteContext,
        args: Mint,
    ) -> ContractResult<Response> {
        if let Err(err) = minter_only_pre(ctx) {
            return Err(err);
        }
        let result = {
            let amount = args.amount;
            { Ok(Response::new()) }
        };
        if let Err(err) = minter_only_post(ctx) {
            return Err(err);
        }
        result
    }
    pub fn minter_only_pre(
        ctx: &::solarsail::ExecuteContext,
    ) -> Result<(), ContractError> {
        let minter = STATE.load(ctx.deps.storage, minter);
        if ctx.info.sender != minter {
            return Err(ContractError::Unauthorized);
        }
        Ok(())
    }
    pub fn minter_only_post(
        ctx: &::solarsail::ExecuteContext,
    ) -> Result<(), ContractError> {
        Ok(())
    }
}
