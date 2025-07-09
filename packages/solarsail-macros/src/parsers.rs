use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{Expr, Ident, LitStr, Variant};
use syn::spanned::Spanned;

/// Custom parser for assignment pairs (key = value, ..)
#[allow(unused)]
pub struct AssignPairs {
  pub pairs: Vec<(Ident, Expr)>,
}

impl syn::parse::Parse for AssignPairs {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let mut pairs = Vec::new();

    while !input.is_empty() {
      let key: Ident = input.parse()?;
      input.parse::<syn::Token![=]>()?;
      let value: Expr = input.parse()?;

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
  pub pairs: Vec<(Ident, Expr)>,
}

impl syn::parse::Parse for KVPairs {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let content;
    syn::braced!(content in input);

    let mut pairs = Vec::new();

    while !content.is_empty() {
      let key: Ident = content.parse()?;
      content.parse::<syn::Token![:]>()?;
      let value: Expr = content.parse()?;

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
  pub name: Ident,
  pub key_type: syn::Type,
  pub value_type: syn::Type,
}

impl syn::parse::Parse for StateMap {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let name: Ident = input.parse()?;
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

/// Custom parser for retrieve macro (either empty or map_name[item_name])
pub enum Retrieve {
  Map {
    map_name: Ident,
    item_name: Expr,
  },
  State {
    store_name: Ident,
  },
}

impl syn::parse::Parse for Retrieve {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    if input.is_empty() {
      return Ok(Retrieve::State {
        store_name: Ident::new("STATE", Span::call_site()),
      });
    }

    let store_name: Ident = input.parse()?;
    if input.peek(syn::token::Bracket) {
      let content;
      syn::bracketed!(content in input);
      let item_name: Expr = content.parse()?;
      Ok(Retrieve::Map {
        map_name: store_name,
        item_name,
      })
    } else {
      Ok(Retrieve::State { store_name })
    }
  }
}

/// Custom parser for persist macro (either expr or map_name[item_name], expr)
pub enum Persist {
  State {
    store_name: Ident,
    value: Expr,
  },
  Map {
    map_name: Ident,
    item_name: Expr,
    value: Expr,
  },
  StructConstruction {
    store_name: Ident,
    value: Expr,
  },
}

impl syn::parse::Parse for Persist {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    // TODO: persist!(StoreName::Variant ...) syntax

    // persist!({ ... }) syntax
    // aka default store syntax
    if input.peek(syn::token::Brace) {
      let value = input.parse::<Expr>()?;
      Ok(Persist::State {
        store_name: Ident::new("STATE", Span::call_site()),
        value,
      })
    }
    // persist!(StoreName { ... }) syntax
    else if input.peek(Ident) && input.peek2(syn::token::Brace) {
      let construct = input.parse::<syn::ExprStruct>()?;
      let store_name = construct.path.get_ident();
      if store_name.is_none() {
        return Err(syn::Error::new(construct.path.span(), "Struct syntax must follow `StoreName { ... }` pattern"));
      }
      Ok(Persist::StructConstruction {
        store_name: store_name.unwrap().clone(),
        value: construct.into(),
      })
    } else {
      // persist!(StoreName = value) syntax
      let store_name = input.parse::<Ident>()?;
      if input.peek(syn::token::Bracket) {
        let item_name;
        syn::bracketed!(item_name in input);
        let item_name: Expr = item_name.parse()?;

        input.parse::<syn::Token![=]>()?;
        let value = input.parse::<Expr>()?;
        Ok(Persist::Map {
          map_name: store_name,
          item_name,
          value,
        })
      }
      // persist!(MapName[item_name] = value) syntax
      else {
        input.parse::<syn::Token![=]>()?;
        let value = input.parse::<Expr>()?;
        Ok(Persist::State {
          store_name,
          value,
        })
      }
    }
  }
}

/// Custom parser for upstate macro
pub enum UpState {
  Store {
    store_name: Ident,
    kvs: KVPairs,
  },
  Map {
    map_name: Ident,
    item_name: Expr,
    kvs: KVPairs,
  },
}

impl syn::parse::Parse for UpState {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    if input.peek(Ident) {
      let store_name: Ident = input.parse()?;

      if input.peek(syn::Token![:]) {
        input.parse::<syn::Token![:]>()?;
        let kvs: KVPairs = input.parse()?;
        Ok(UpState::Store {
          store_name,
          kvs,
        })
      } else {
        let content;
        syn::bracketed!(content in input);
        let item_name: Expr = content.parse()?;

        input.parse::<syn::Token![:]>()?;

        let kvs: KVPairs = content.parse()?;
        Ok(UpState::Map {
          map_name: store_name,
          item_name,
          kvs,
        })
      }
    } else {
      let kvs: KVPairs = input.parse()?;
      Ok(UpState::Store {
        store_name: Ident::new("STATE", Span::call_site()),
        kvs,
      })
    }
  }
}

/// Custom parser for state macro (optional identifier, fields)
pub struct State {
  pub identifier: Option<Ident>,
  pub fields: syn::FieldsNamed,
}

impl syn::parse::Parse for State {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    // support custom state identifier
    let identifier = if input.peek(Ident) && input.peek2(syn::Token![,]) {
      let ident = Some(input.parse()?);
      input.parse::<syn::Token![,]>()?;
      ident
    } else if input.peek(Ident) && input.peek2(syn::token::Brace) {
      Some(input.parse()?)
    } else {
      None
    };

    // Parse the fields
    let fields: syn::FieldsNamed = input.parse()?;

    Ok(State { identifier, fields })
  }
}

/// Custom parser for assert macro (condition, error)
pub struct Assert {
  pub condition: Expr,
  pub error: Expr,
}

impl syn::parse::Parse for Assert {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let condition: Expr = input.parse()?;
    input.parse::<syn::Token![,]>()?;
    let error: Expr = input.parse()?;

    Ok(Assert { condition, error })
  }
}

/// Custom parser for enumerate macro (MapName[prefixes...], min..max, descending)
pub struct Enumerate {
  pub map_name: Ident,
  pub prefixes: Vec<Expr>,
  pub bounds: EnumerateBounds,
  pub order: cosmwasm_std::Order,
}

#[derive(Default)]
pub struct EnumerateBounds {
  pub start: Option<Expr>,
  pub end: Option<Expr>,
  pub closed: bool,
}

impl syn::parse::Parse for Enumerate {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let map_name: Ident = input.parse()?;
    let mut bounds: EnumerateBounds = Default::default();
    let mut order = cosmwasm_std::Order::Ascending;

    // optional prefixes in brackets
    let prefixes = if MapIndex::peek(input) {
      input.parse::<MapIndex>()?.indexes
    } else {
      vec![]
    };

    while !input.is_empty() {
      input.parse::<syn::Token![,]>()?;
      let expr: Expr = input.parse()?;
      match expr {
        Expr::Range(range) => {
          bounds.start = range.start.map(|expr| *expr.clone());
          bounds.end = range.end.map(|expr| *expr.clone());
          bounds.closed = matches!(range.limits, syn::RangeLimits::Closed(_));
        }
        Expr::Path(path) if path.path.is_ident("descending") => {
          order = cosmwasm_std::Order::Descending;
        }
        _ => {
          return Err(syn::Error::new(expr.span(), "Unknown expression"));
        }
      }
    }

    Ok(Enumerate {
      map_name,
      prefixes,
      bounds,
      order,
    })
  }
}

pub struct MapIndex {
  pub indexes: Vec<Expr>,
}

impl syn::parse::Parse for MapIndex {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let content;
    syn::bracketed!(content in input);

    let mut indexes = Vec::new();
    while !content.is_empty() {
      let index: Expr = content.parse()?;
      indexes.push(index);
      if !content.is_empty() {
        content.parse::<syn::Token![,]>()?;
      }
    }
    Ok(MapIndex { indexes })
  }
}

impl MapIndex {
  /// Peek if the parse stream suggests a potential map index at the current position
  pub fn peek(input: syn::parse::ParseStream) -> bool {
    input.peek(syn::token::Bracket)
  }
}

pub struct ErrorDef {
  pub expr: Variant,
  pub msg: String,
}

impl syn::parse::Parse for ErrorDef {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let expr: Variant = input.parse()?;
    let msg = if input.peek(syn::Token![,]) {
      input.parse::<syn::Token![,]>()?;
      input.parse::<LitStr>()?.value()
    } else {
      String::new()
    };
    Ok(ErrorDef { expr, msg })
  }
}

impl ToTokens for ErrorDef {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let msg = if self.msg.is_empty() {
      self.expr.ident.to_string()
    } else {
      self.msg.clone()
    };

    let var = self.expr.clone();

    tokens.extend(quote! {
      #[error(#msg)] #var
    });
  }
}

/// Helper parser for `emit!` macro
pub struct EmitEvent {
  pub name: String,
  pub attrs: EventAttributes,
}

impl syn::parse::Parse for EmitEvent {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let name = if input.peek(Ident) {
      input.parse::<Ident>()?.to_string()
    } else {
      input.parse::<LitStr>()?.value()
    };

    let attrs = if !input.is_empty() {
      input.parse::<syn::Token![,]>()?;
      input.parse::<EventAttributes>()?
    } else {
      EventAttributes { pairs: vec![] }
    };

    Ok(EmitEvent { name, attrs })
  }
}

impl ToTokens for EmitEvent {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let name = &self.name;
    let name = quote! { format!("{}:{}", env!("CARGO_PKG_NAME"), #name) };

    let attrs = self.attrs.pairs
      .iter()
      .map(|(key, value)| {
        quote! { .add_attribute(#key, #value) }
      })
      .collect::<Vec<_>>();

    tokens.extend(quote! {
      __solarsail_events.push(::cosmwasm_std::Event::new(#name)#(#attrs)*)
    });
  }
}

pub struct EventAttributes {
  pub pairs: Vec<(String, Expr)>,
}

impl syn::parse::Parse for EventAttributes {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let content;
    syn::braced!(content in input);

    let mut pairs = Vec::new();
    while !content.is_empty() {
      let span = content.span();
      let key = if content.peek(Ident) {
        content.parse::<Ident>()?.to_string()
      } else {
        content.parse::<LitStr>()?.value()
      };

      let value = if content.peek(syn::Token![:]) {
        content.parse::<syn::Token![:]>()?;
        content.parse::<Expr>()?
      } else {
        let ident = Ident::new(&key, span);
        syn::parse2::<Expr>(quote! { #ident })?
      };

      if !content.is_empty() {
        content.parse::<syn::Token![,]>()?;
      }

      pairs.push((key, value));
    }

    Ok(EventAttributes { pairs })
  }
}
