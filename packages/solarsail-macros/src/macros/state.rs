use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

pub type StateMacroInput = crate::parsers::State;

pub fn state(input: &StateMacroInput) -> TokenStream {
  let fields = &input.fields.named;

  // `authority` fields have their own `Item`s and are not included in the `State` struct
  let field_definitions: Vec<_> = fields
    .iter()
    .filter(|field| {
      !field.attrs.iter().any(|attr| attr.path().is_ident("authority"))
    })
    .map(|field| {
      let ident = &field.ident;
      let ty = &field.ty;

      quote! {
        pub #ident: #ty,
      }
    })
    .collect();

  let struct_name = input.identifier.as_ref()
    .map(|ident| {
      Ident::new(&ident.to_string().to_case(Case::Pascal), ident.span())
    })
    .unwrap_or_else(|| Ident::new("State", Span::call_site()));

  let storage_name = input.identifier.as_ref()
    .map(|ident| Ident::new(
      &ident.to_string().to_case(Case::UpperSnake),
      ident.span()))
    .unwrap_or_else(|| Ident::new("STATE", Span::call_site()));

  quote! {
    #[cosmwasm_schema::cw_serde]
    pub struct #struct_name {
      #(#field_definitions)*
    }

    pub const #storage_name: ::cw_storage_plus::Item<#struct_name> = ::cw_storage_plus::Item::new("state");
  }
}

pub fn state_map(input: &crate::parsers::StateMap) -> TokenStream {
  let name = &input.name;
  let key_type = &input.key_type;
  let value_type = &input.value_type;

  // Convert the name to uppercase for the constant name
  let const_name = Ident::new(
    &name.to_string().to_uppercase(),
    name.span(),
  );

  quote! {
    pub const #const_name: ::cw_storage_plus::Map<#key_type, #value_type> = ::cw_storage_plus::Map::new(stringify!(#name));
  }
}
