use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Field, Ident};
use syn::spanned::Spanned;

pub struct AuthorityInfo {
  pub authority_fields_required: Vec<syn::Ident>,
  pub authority_fields_optional: Vec<syn::Ident>,
}

/// Extract authority fields from state fields and generate authority-related code
pub fn extract_authority_fields(fields: &[Field]) -> Result<AuthorityInfo, syn::Error> {
  let mut authority_fields_required = Vec::new();
  let mut authority_fields_optional = Vec::new();

  for field in fields {
    let has_authority_attr = field.attrs.iter().any(|attr| {
      attr.path().is_ident("authority")
    });

    if has_authority_attr {
      let field_name = field.ident.as_ref().ok_or_else(|| {
        syn::Error::new(field.span(), "Authority field must have a name")
      })?;

      // Validate that the field type is Addr or Option<Addr>
      let ty = &field.ty;
      let ty_name = format!("{}", quote! { #ty }.to_string());

      if ty_name == "Addr" {
        authority_fields_required.push(field_name.clone());
      } else if [
        "Option < Addr >",
        "Option < cosmwasm_std :: Addr >",
        "Option < :: cosmwasm_std :: Addr >",
      ].contains(&ty_name.as_str()) {
        authority_fields_optional.push(field_name.clone());
      } else {
        return Err(syn::Error::new(
          field.ty.span(),
          "Authority fields must be of type Addr or Option<Addr>"
        ));
      }
    }
  }

  Ok(AuthorityInfo {
    authority_fields_required,
    authority_fields_optional,
  })
}

/// Generate the TransferAuthority enum and transfer_authority function
pub fn generate_authority_code(authority_info: &AuthorityInfo) -> TokenStream {
  let total_fields =
    authority_info.authority_fields_required.len() + authority_info.authority_fields_optional.len();
  if total_fields == 0 {
    return quote! {};
  }

  // generate `Item`s for each authority field
  let codegen = |option: bool| move |field_name: &Ident| {
    let store_name = Ident::new(
      &format!("__SS_AUTH_{}", field_name.to_string().to_uppercase()),
      field_name.span(),
    );
    let ty = match option {
      true => quote! { Option<Addr> },
      false => quote! { Addr },
    };
    let key = format!("ss_auth_{}", field_name);
    quote! {
      const #store_name: ::cw_storage_plus::Item<#ty> = ::cw_storage_plus::Item::new(#key);
    }
  };

  let required_storage_items: Vec<_> = authority_info.authority_fields_required
    .iter()
    .map(codegen(false))
    .collect();

  let optional_storage_items: Vec<_> = authority_info.authority_fields_optional
    .iter()
    .map(codegen(true))
    .collect();

  // generate variants for TransferAuthority (payload-carrying) enum
  let codegen = |option: bool| move |field_name: &Ident| {
    let variant_name = Ident::new(
      &field_name.to_string().to_case(Case::Pascal),
      field_name.span(),
    );
    let ty = match option {
      true => quote! { Option<String> },
      false => quote! { String },
    };
    quote! {
      #variant_name(#ty),
    }
  };

  let required_variants_transfer = authority_info.authority_fields_required
    .iter()
    .map(codegen(false))
    .collect::<Vec<_>>();

  let optional_variants_transfer = authority_info.authority_fields_optional
    .iter()
    .map(codegen(true))
    .collect::<Vec<_>>();

  // generate variants for Authority (payload-less) enum
  let codegen = |field_name: &Ident| {
    let variant_name = Ident::new(
      &field_name.to_string().to_case(Case::Pascal),
      field_name.span(),
    );
    quote! { #variant_name, }
  };

  let authority_variants = authority_info.authority_fields_required
    .iter()
    .chain(authority_info.authority_fields_optional.iter())
    .map(codegen)
    .collect::<Vec<_>>();

  // generate match arms for execute_transfer_authority function (uses TransferAuthority)
  let codegen = |option: bool| move |field_name: &Ident| {
    let variant_name = syn::Ident::new(
      &field_name.to_string().to_case(Case::Pascal),
      field_name.span(),
    );
    let store_name = syn::Ident::new(
      &format!("__SS_AUTH_{}", field_name.to_string().to_uppercase()),
      field_name.span(),
    );

    let mut auth_check = quote! { if ctx.info.sender != current_auth { return Err(ContractError::Unauthorized); } };
    if option {
      auth_check = quote! {
        match current_auth {
          Some(current_auth) => #auth_check,
          None => return Err(ContractError::Unauthorized),
        }
      };
    }

    let mut validate_addr = quote! { ctx.deps.api.addr_validate(&new_authority)? };
    if option {
      validate_addr = quote! { new_authority.as_ref().map(|new_authority| Ok::<_, cosmwasm_std::StdError>(#validate_addr)).transpose()? };
    }

    let variant_name_str = variant_name.to_string();
    let event = match option {
      true => quote! {
        match new_authority {
          Some(new_authority) => ::cosmwasm_std::Event::new("transfer_authority")
            .add_attribute("authority", #variant_name_str)
            .add_attribute("address", new_authority.to_string()),
          None => ::cosmwasm_std::Event::new("renounce_authority")
            .add_attribute("authority", #variant_name_str)
        }
      },
      false => quote! {
        ::cosmwasm_std::Event::new("transfer_authority")
          .add_attribute("authority", #variant_name_str)
          .add_attribute("address", new_authority.to_string())
      }
    };

    quote! {
      TransferAuthority::#variant_name(new_authority) => {
        let current_auth = #store_name.load(ctx.deps.storage)?;
        #auth_check;

        let new_addr = #validate_addr;
        #store_name.save(ctx.deps.storage, &new_addr)?;
        Ok(::cosmwasm_std::Response::new().add_event(#event))
      }
    }
  };

  let required_match_arms: Vec<_> = authority_info.authority_fields_required
    .iter()
    .map(codegen(false))
    .collect();

  let optional_match_arms: Vec<_> = authority_info.authority_fields_optional
    .iter()
    .map(codegen(true))
    .collect();

  // generate match arms for query_authority function (reads, no checks)
  let codegen = |option: bool| move |field_name: &Ident| {
    let variant_name = syn::Ident::new(
      &field_name.to_string().to_case(Case::Pascal),
      field_name.span(),
    );
    let store_name = syn::Ident::new(
      &format!("__SS_AUTH_{}", field_name.to_string().to_uppercase()),
      field_name.span(),
    );

    if option {
      quote! {
        Authority::#variant_name => {
          let current_auth = #store_name.load(ctx.deps.storage)?;
          Ok(current_auth)
        }
      }
    } else {
      quote! {
        Authority::#variant_name => {
          let current_auth = #store_name.load(ctx.deps.storage)?;
          Ok(Some(current_auth))
        }
      }
    }
  };

  let required_query_arms: Vec<_> = authority_info.authority_fields_required
    .iter()
    .map(codegen(false))
    .collect();

  let optional_query_arms: Vec<_> = authority_info.authority_fields_optional
    .iter()
    .map(codegen(true))
    .collect();

  quote! {
    #(#required_storage_items)*
    #(#optional_storage_items)*

    #[::cosmwasm_schema::cw_serde]
    pub enum TransferAuthority {
      #(#required_variants_transfer)*
      #(#optional_variants_transfer)*
    }

    #[::cosmwasm_schema::cw_serde]
    pub enum Authority {
      #(#authority_variants)*
    }

    pub fn query_authority(ctx: ::solarsail::QueryContext, authority: Authority) -> ::std::result::Result<Option<::cosmwasm_std::Addr>, ContractError> {
      match authority {
        #(#required_query_arms)*
        #(#optional_query_arms)*
      }
    }

    pub fn execute_transfer_authority(ctx: ::solarsail::ExecuteContext, authority: TransferAuthority) -> ::std::result::Result<::cosmwasm_std::Response, ContractError> {
      match authority {
        #(#required_match_arms)*
        #(#optional_match_arms)*
      }
    }

    pub fn check_authority(ctx: &::solarsail::ExecuteContext, authority: Authority) -> ::std::result::Result<(), ContractError> {
      let qctx = ::solarsail::QueryContext::new(ctx.deps.as_ref(), ctx.env.clone());
      let current = query_authority(qctx, authority)?;
      match current {
        Some(current_auth) if current_auth == ctx.info.sender => Ok(()),
        _ => Err(ContractError::Unauthorized),
      }
    }
  }
}
