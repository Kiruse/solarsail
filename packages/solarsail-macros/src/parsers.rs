/// Custom parser for assignment pairs (key = value, ..)
pub struct AssignPairs {
  pub pairs: Vec<(syn::Ident, syn::Expr)>,
}

impl syn::parse::Parse for AssignPairs {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let mut pairs = Vec::new();

    while !input.is_empty() {
      let key: syn::Ident = input.parse()?;
      input.parse::<syn::Token![=]>()?;
      let value: syn::Expr = input.parse()?;

      pairs.push((key, value));

      if input.is_empty() {
        break;
      }
      input.parse::<syn::Token![,]>()?;
    }

    Ok(AssignPairs { pairs })
  }
}

/// Custom parser for struct-like key-value pairs ({ key: value, .. })
pub struct KVPairs {
  pub pairs: Vec<(syn::Ident, syn::Expr)>,
}

impl syn::parse::Parse for KVPairs {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let content;
    syn::braced!(content in input);

    let mut pairs = Vec::new();

    while !content.is_empty() {
      let key: syn::Ident = content.parse()?;
      content.parse::<syn::Token![:]>()?;
      let value: syn::Expr = content.parse()?;

      pairs.push((key, value));

      if content.is_empty() {
        break;
      }
      content.parse::<syn::Token![,]>()?;
    }

    Ok(KVPairs { pairs })
  }
}
