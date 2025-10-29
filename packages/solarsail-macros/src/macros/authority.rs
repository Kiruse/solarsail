use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::punctuated::Punctuated;
use syn::{Block, Ident, parse_quote};

use crate::macros::utils::rename_ident;

pub fn transform(mut item: syn::ItemFn, authorities: &[&Ident]) -> Result<TokenStream, syn::Error> {
  let authorities = authorities
    .iter()
    .map(|auth| rename_ident!(Case::Pascal, "{}", auth))
    .collect::<Vec<_>>();
  let block: Block = parse_quote! {{
    if ![#(Authority::#authorities.check(&ctx)),*].iter().any(|auth| auth.is_ok()) {
      return Err(solarsail::authority::AuthorityError::Unauthorized.into());
    }
  }};
  item.block.stmts.splice(0..0, block.stmts);
  Ok(item.to_token_stream())
}

/// Generate the AuthorityTransfer enum and transfer_authority function
pub fn generate_authority_code(items: &Punctuated<Ident, syn::Token![,]>) -> TokenStream {
  if items.len() == 0 {
    return quote! {};
  }

  let fields = items.iter().cloned().collect::<Vec<_>>();

  let variants = fields
    .iter()
    .map(|field| Ident::new(
      &field.to_string().to_case(Case::Pascal),
      field.span(),
    ))
    .collect::<Vec<_>>();

  quote! {
    struct AuthorityStorage {
      #(#fields: solarsail::cw_storage_plus::Item<solarsail::authority::AuthorityState>),*
    }

    impl AuthorityStorage {
      pub fn new() -> Self {
        Self {
          #(#fields: solarsail::cw_storage_plus::Item::new(stringify!(#fields))),*
        }
      }
    }

    #[solarize]
    pub enum Authority {
      #(#variants),*
    }

    use solarsail::authority::Authority as AuthorityTrait;

    impl AuthorityTrait for Authority {
      fn storage(&self) -> solarsail::authority::Storage {
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

    #[solarize]
    pub enum AuthorityOperation {
      Transfer {
        authority: Authority,
        addr: Option<solarsail::cw_std::Addr>,
        expires: solarsail::cw_utils::Expiration,
      },
      Accept {
        authority: Authority,
      },
    }

    impl AuthorityOperation {
      pub fn handle(&self, ctx: &mut ExecuteContext) -> Result<(), solarsail::authority::AuthorityError> {
        match self {
          AuthorityOperation::Transfer { authority, addr, expires } => {
            authority.transfer(ctx, addr.clone(), expires.clone())?;
          }
          AuthorityOperation::Accept { authority } => {
            authority.accept_transfer(ctx)?;
          }
        }
        Ok(())
      }
    }
  }
}
