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

/// Custom parser for state_map macro (name = key_type => value_type)
pub struct StateMap {
  pub name: syn::Ident,
  pub key_type: syn::Type,
  pub value_type: syn::Type,
}

impl syn::parse::Parse for StateMap {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let name: syn::Ident = input.parse()?;
    input.parse::<syn::Token![:]>()?;
    let key_type: syn::Type = input.parse()?;
    input.parse::<syn::Token![=>]>()?;
    let value_type: syn::Type = input.parse()?;

    Ok(StateMap {
      name,
      key_type,
      value_type,
    })
  }
}

/// Custom parser for rstate macro (either empty or map_name[item_name])
pub struct RState {
  pub map_name: Option<syn::Ident>,
  pub item_name: Option<syn::Expr>,
}

impl syn::parse::Parse for RState {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    if input.is_empty() {
      return Ok(RState {
        map_name: None,
        item_name: None,
      });
    }

    let map_name: syn::Ident = input.parse()?;
    let content;
    syn::bracketed!(content in input);
    let item_name: syn::Expr = content.parse()?;

    Ok(RState {
      map_name: Some(map_name),
      item_name: Some(item_name),
    })
  }
}

/// Custom parser for wstate macro (either expr or map_name[item_name], expr)
pub struct WState {
  pub map_name: Option<syn::Ident>,
  pub item_name: Option<syn::Expr>,
  pub value: syn::Expr,
}

impl syn::parse::Parse for WState {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    // Try to parse as map_name[item_name], value first
    if let Ok(variant) = input.parse::<WStateMapVariant>() {
      return Ok(WState {
        map_name: Some(variant.0),
        item_name: Some(variant.1),
        value: variant.2,
      });
    }

    // If that fails, try to parse as just a value
    let value: syn::Expr = input.parse()?;
    Ok(WState {
      map_name: None,
      item_name: None,
      value,
    })
  }
}

/// Helper struct for parsing map variant of wstate
struct WStateMapVariant(syn::Ident, syn::Expr, syn::Expr);

impl syn::parse::Parse for WStateMapVariant {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let map_name: syn::Ident = input.parse()?;
    let content;
    syn::bracketed!(content in input);
    let item_name: syn::Expr = content.parse()?;
    input.parse::<syn::Token![,]>()?;
    let value: syn::Expr = input.parse()?;
    Ok(WStateMapVariant(map_name, item_name, value))
  }
}

/// Custom parser for upstate macro (either { kvs } or map_name[item_name], { kvs })
pub struct UpState {
  pub map_name: Option<syn::Ident>,
  pub item_name: Option<syn::Expr>,
  pub kvs: KVPairs,
}

impl syn::parse::Parse for UpState {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    // Try to parse as map_name[item_name], { kvs } first
    if let Ok(variant) = input.parse::<UpStateMapVariant>() {
      return Ok(UpState {
        map_name: Some(variant.0),
        item_name: Some(variant.1),
        kvs: variant.2,
      });
    }

    // If that fails, try to parse as just { kvs }
    let kvs: KVPairs = input.parse()?;
    Ok(UpState {
      map_name: None,
      item_name: None,
      kvs,
    })
  }
}

/// Helper struct for parsing map variant of upstate
struct UpStateMapVariant(syn::Ident, syn::Expr, KVPairs);

impl syn::parse::Parse for UpStateMapVariant {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let map_name: syn::Ident = input.parse()?;
    let content;
    syn::bracketed!(content in input);
    let item_name: syn::Expr = content.parse()?;
    input.parse::<syn::Token![,]>()?;
    let kvs: KVPairs = input.parse()?;
    Ok(UpStateMapVariant(map_name, item_name, kvs))
  }
}
