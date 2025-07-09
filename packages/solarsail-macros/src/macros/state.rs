use proc_macro2::TokenStream;
use quote::quote;
use syn::FieldsNamed;

pub fn state(input: &FieldsNamed) -> TokenStream {
  let fields = &input.named;
  let field_definitions: Vec<_> = fields.iter().map(|field| {
    let attrs = &field.attrs;
    let ident = &field.ident;
    let ty = &field.ty;

    // Check if this field has the authority attribute
    let has_authority = attrs.iter().any(|attr| {
      attr.path().is_ident("authority")
    });

    if has_authority {
      // Generate authority field with validation
      quote! {
        pub #ident: #ty,
        // Authority validation will be handled by the contract macro
      }
    } else {
      quote! {
        pub #ident: #ty,
      }
    }
  }).collect();

  let struct_name = syn::Ident::new("State", proc_macro2::Span::call_site());

  let expanded = quote::quote! {
    #[cosmwasm_schema::cw_serde]
    pub struct #struct_name {
      #(#field_definitions)*
    }

    pub const STATE: ::cw_storage_plus::Item<#struct_name> = ::cw_storage_plus::Item::new("state");
  };

  TokenStream::from(expanded)
}

pub fn state_map(input: &crate::parsers::StateMap) -> TokenStream {
  let name = &input.name;
  let key_type = &input.key_type;
  let value_type = &input.value_type;

  // Convert the name to uppercase for the constant name
  let const_name = syn::Ident::new(
    &name.to_string().to_uppercase(),
    name.span(),
  );

  let expanded = quote! {
    pub const #const_name: ::cw_storage_plus::Map<#key_type, #value_type> = ::cw_storage_plus::Map::new(stringify!(#name));
  };

  TokenStream::from(expanded)
}
