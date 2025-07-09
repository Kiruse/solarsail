use proc_macro2::TokenStream;
use quote::quote;
use syn::{Field, FieldsNamed, Type, Error};
use syn::spanned::Spanned;

pub fn state(input: &FieldsNamed) -> TokenStream {
  let fields = &input.named;

  let mut authority_fields: Vec<&Field> = Vec::new();
  let mut errors = Vec::new();

  let field_definitions: Vec<_> = fields.iter().map(|field| {
    let attrs = &field.attrs;
    let ident = &field.ident;
    let ty = &field.ty;

    // Check if this field has the authority attribute
    let has_authority = attrs.iter().any(|attr| {
      attr.path().is_ident("authority")
    });

    if has_authority {
      authority_fields.push(field);
    }

    quote! {
      pub #ident: #ty,
    }
  }).collect();

  let struct_name = syn::Ident::new("State", proc_macro2::Span::call_site());

  let authority_setters: Vec<TokenStream> = authority_fields.iter().map(|field| {
    let ident = &field.ident.as_ref().unwrap();
    let ty = &field.ty;
    let ty_ident = get_type_ident(ty);

    // Assert that the field type is either String or Addr
    let is_valid_type = match ty_ident {
      Some(ident) => ident.to_string() == "String" || ident.to_string() == "Addr",
      _ => false,
    };

    if !is_valid_type {
      let error = Error::new(
        ty.span(),
        format!("Authority field '{}' must have type String or Addr", ident),
      );
      errors.push(error);
    }

    let fn_name = syn::Ident::new(
      format!("transfer_{}", ident).as_str(),
      proc_macro2::Span::call_site(),
    );

    let mut addr_check = quote! {
      ctx.deps.api.addr_validate(&value)?;
    };

    if let Some(ident) = ty_ident {
      if ident.to_string() == "Addr" {
        addr_check = quote! {
          if !value.is_empty() {
            #addr_check;
          }
        };
      }
    }

    quote! {
      pub fn #fn_name(
        ctx: &mut ::solarsail::ExecuteContext,
        value: String,
      ) -> Result<::cosmwasm_std::Response, ContractError> {
        let curr_authority = rstate!()?.#ident;
        if curr_authority != ctx.info.sender {
          return Err(ContractError::Unauthorized);
        }

        #addr_check;

        upstate!({
          #ident: value.clone(),
        })?;

        Ok(Response::new()
          .add_attribute("action", stringify!(#fn_name))
          .add_attribute("new_authority", &value)
        )
      }
    }
  }).collect();

  // If there are any errors, return them as compile errors
  if !errors.is_empty() {
    let error_tokens: Vec<TokenStream> = errors.into_iter().map(|e| e.to_compile_error()).collect();
    return error_tokens.first().unwrap().clone();
  }

  quote::quote! {
    #[cosmwasm_schema::cw_serde]
    pub struct #struct_name {
      #(#field_definitions)*
    }

    pub const STATE: ::cw_storage_plus::Item<#struct_name> = ::cw_storage_plus::Item::new("state");

    #(#authority_setters)*
  }
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

fn get_type_ident(ty: &Type) -> Option<&syn::Ident> {
  match ty {
    Type::Path(type_path) => Some(&type_path.path.segments.last().unwrap().ident),
    _ => None,
  }
}
