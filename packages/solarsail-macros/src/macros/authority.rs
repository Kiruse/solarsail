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

/// Generate the AuthorityTransfer enum and transfer_authority function
pub fn generate_authority_code(authority_info: &AuthorityInfo) -> TokenStream {
  let total_fields =
    authority_info.authority_fields_required.len() + authority_info.authority_fields_optional.len();
  if total_fields == 0 {
    return quote! {};
  }

  let fields = authority_info.authority_fields_required
    .iter()
    .chain(authority_info.authority_fields_optional.iter())
    .collect::<Vec<_>>();

  let variants = fields
    .iter()
    .map(|field| Ident::new(
      &field.to_string().to_case(Case::Pascal),
      field.span(),
    ))
    .collect::<Vec<_>>();

  quote! {
    struct AuthorityStorage {
      #(#fields: ::cw_storage_plus::Item<::solarsail::authority::AuthorityState>),*
    }

    impl AuthorityStorage {
      pub fn new() -> Self {
        Self {
          #(#fields: ::cw_storage_plus::Item::new(stringify!(#fields))),*
        }
      }
    }

    pub enum Authority {
      #(#variants),*
    }

    use ::solarsail::authority::Authority as AuthorityTrait;

    impl AuthorityTrait for Authority {
      fn storage(&self) -> ::solarsail::authority::Storage {
        match self {
          #(Authority::#variants => AuthorityStorage::new().#fields),*
        }
      }

      fn name(&self) -> &str {
        match self {
          #(Authority::#variants => stringify!(#fields)),*
        }
      }
    }

    pub enum AuthorityTransfer {
      #(#variants {
        addr: Option<::cosmwasm_std::Addr>,
        expires: ::solarsail::authority::Expiration,
      }),*
    }

    use ::solarsail::authority::AuthorityTransfer as AuthorityTransferTrait;

    impl AuthorityTransferTrait for AuthorityTransfer {
      type Authority = Authority;

      fn addr(&self) -> &Option<::cosmwasm_std::Addr> {
        match self {
          #(AuthorityTransfer::#variants { addr, .. } => addr),*
        }
      }

      fn expires(&self) -> &::solarsail::authority::Expiration {
        match self {
          #(AuthorityTransfer::#variants { expires, .. } => expires),*
        }
      }

      fn authority(&self) -> Authority {
        match self {
          #(AuthorityTransfer::#variants { .. } => Authority::#variants),*
        }
      }
    }
  }
}
