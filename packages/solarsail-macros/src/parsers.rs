use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Expr, FieldsNamed, Ident, ItemEnum, ItemFn, ItemImpl, ItemStruct, LitStr, Token, Variant, parse_quote};
use syn::spanned::Spanned;

use crate::macros::utils::make_ident;

pub struct ContractDef {
  pub name: Ident,
  /// Placeholder for a future feature.
  pub extends: Option<Ident>,
  /// Placeholder for a future feature.
  pub implements: Punctuated<Ident, syn::Token![,]>,
  pub authority: Punctuated<Ident, syn::Token![,]>,
  pub state_defs: Vec<StateDef>,
  pub errors: Vec<ErrorDef>,
}

impl Parse for ContractDef {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let mut name: Option<Ident> = None;
    let mut extends: Option<Ident> = None;
    let mut implements: Punctuated<Ident, syn::Token![,]> = Punctuated::new();
    let mut authority: Punctuated<Ident, syn::Token![,]> = Punctuated::new();
    let mut state_defs: Vec<StateDef> = Vec::new();
    let mut errors: Vec<ErrorDef> = Vec::new();

    let consume_pseudo_macro = || {
      input.parse::<Ident>()?;
      input.parse::<syn::Token![!]>()?;
      Ok::<(), syn::Error>(())
    };

    while !input.is_empty() {
      let peeked = peek_ident(input)?;
      match peeked.to_string().as_str() {
        "name" => {
          consume_pseudo_macro()?;
          if name.is_none() {
            name = Some(input.parse()?);
          } else {
            return Err(syn::Error::new(peeked.span(), "Name already defined"));
          }
          input.parse::<syn::Token![;]>()?;
        }
        AuthorityDef::KEYWORD => {
          consume_pseudo_macro()?;
          let auth_def: AuthorityDef = input.parse()?;
          authority.extend(auth_def.authorities);
          input.parse::<syn::Token![;]>()?;
        }
        StateDef::KEYWORD => {
          consume_pseudo_macro()?;
          let state_def: StateDef = input.parse()?;
          if matches!(state_def, StateDef::Map(_)) {
            input.parse::<syn::Token![;]>()?;
          }
          state_defs.push(state_def);
        }
        ErrorDef::KEYWORD => {
          consume_pseudo_macro()?;
          let error_def: ErrorDef = input.parse()?;
          errors.push(error_def);
          input.parse::<syn::Token![;]>()?;
        }
        "extends" => {
          consume_pseudo_macro()?;
          extends = Some(input.parse()?);
          input.parse::<syn::Token![;]>()?;
        }
        "implements" => {
          consume_pseudo_macro()?;
          let tmp = input.parse_terminated(Ident::parse, syn::Token![,])?;
          if tmp.is_empty() {
            return Err(syn::Error::new(peeked.span(), "`implements` must have at least one interface"));
          }
          implements.extend(tmp);
          input.parse::<syn::Token![;]>()?;
        }
        _ => {
          return Err(syn::Error::new(peeked.span(), "Unknown keyword"));
        }
      }
    }

    Ok(Self {
      name: name.ok_or(syn::Error::new(Span::call_site(), "Name is required"))?,
      extends,
      implements,
      authority,
      state_defs,
      errors,
    })
  }
}

/// Custom parser for authority definition within contract definition
pub struct AuthorityDef {
  pub authorities: Punctuated<Ident, syn::Token![,]>,
}

impl Parse for AuthorityDef {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let content;
    syn::bracketed!(content in input);
    let authorities = content.parse_terminated(Ident::parse, syn::Token![,])?;
    if authorities.is_empty() {
      return Err(syn::Error::new(content.span(), "`authority` must have at least one authority"));
    }
    Ok(AuthorityDef { authorities })
  }
}

impl AuthorityDef {
  pub const KEYWORD: &str = "authority";
}

/// Custom parser for error definition within contract definition
pub struct ErrorDef {
  pub expr: Variant,
  pub msg: String,
}

impl Parse for ErrorDef {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let expr: Variant = input.parse()?;
    let msg = if input.peek(syn::Token![:]) {
      input.parse::<syn::Token![:]>()?;
      input.parse::<LitStr>()?.value()
    } else {
      expr.ident.to_string()
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

impl ErrorDef {
  pub const KEYWORD: &str = "error";
}

/// Custom parser for state macro (store or map)
pub enum StateDef {
  Integrated(StateIntegrated),
  Store(StateStore),
  Map(StateMap),
}

impl Parse for StateDef {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let name: Ident = input.parse()?;

    if input.peek(syn::Token![:]) {
      input.parse::<syn::Token![:]>()?;
      let key_type: syn::Type = input.parse()?;

      input.parse::<syn::Token![=>]>()?;

      let value_type: syn::Type = input.parse()?;

      let indexes = if input.peek(syn::Token![,]) {
        input.parse::<syn::Token![,]>()?;
        let content;
        syn::bracketed!(content in input);
        content.parse_terminated(StateMapIndex::parse, syn::Token![,])?
      } else {
        Punctuated::new()
      };

      Ok(StateDef::Map(StateMap { name, key_type, value_type, indexes }))
    } else if input.peek(syn::Token![=]) {
      input.parse::<syn::Token![=]>()?;
      let ty: syn::Type = input.parse()?;
      input.parse::<syn::Token![;]>()?;
      Ok(StateDef::Integrated(StateIntegrated { name, ty }))
    } else {
      let fields: syn::FieldsNamed = input.parse()?;
      Ok(StateDef::Store(StateStore { name, fields }))
    }
  }
}

impl ToTokens for StateDef {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let append = match self {
      StateDef::Integrated(integrated) => crate::macros::state::state_integrated(integrated),
      StateDef::Store(store) => crate::macros::state::state_store(store),
      StateDef::Map(map) => crate::macros::state::state_map(map),
    };
    tokens.extend(append);
  }
}

impl StateDef {
  pub const KEYWORD: &str = "state";
}

pub struct StateIntegrated {
  pub name: Ident,
  pub ty: syn::Type,
}

pub struct StateStore {
  pub name: Ident,
  pub fields: syn::FieldsNamed,
}

pub struct StateMap {
  pub name: Ident,
  pub key_type: syn::Type,
  pub value_type: syn::Type,
  pub indexes: Punctuated<StateMapIndex, syn::Token![,]>,
}

pub struct StateMapIndex {
  pub field: Ident,
  pub ty: syn::Type,
  pub unique: Option<syn::Token![!]>,
}

impl Parse for StateMapIndex {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let field: Ident = input.parse()?;
    let unique = if input.peek(syn::Token![!]) {
      Some(input.parse()?)
    } else {
      None
    };
    input.parse::<syn::Token![:]>()?;
    let ty: syn::Type = input.parse()?;
    Ok(StateMapIndex { field, ty, unique })
  }
}

pub struct ReturnsArgs {
  pub ty: FieldsNamed,
}

impl Parse for ReturnsArgs {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let ty: FieldsNamed = input.parse()?;
    Ok(ReturnsArgs { ty })
  }
}

/// Custom parser for `response!` macro (`response! { key: value, .. }`), i.e.
/// a `KVPairs` without the braces.
pub struct Response {
  pub pairs: Punctuated<KVPair, syn::Token![,]>,
}

impl Parse for Response {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let pairs = input.parse_terminated(KVPair::parse, syn::Token![,])?;
    Ok(Response { pairs })
  }
}

impl ToTokens for Response {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let pairs = &self.pairs.iter().collect::<Vec<_>>();
    tokens.extend(quote! { { #(#pairs),* } });
  }
}

/// Custom parser for struct-like key-value pairs (`{ key: value, .. }`)
pub struct KVPairs {
  pub pairs: Punctuated<KVPair, syn::Token![,]>,
  #[allow(unused)]
  pub brace: syn::token::Brace,
}

impl Parse for KVPairs {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let content;
    let brace = syn::braced!(content in input);
    let pairs = content.parse_terminated(KVPair::parse, syn::Token![,])?;
    Ok(KVPairs { pairs, brace })
  }
}

impl ToTokens for KVPairs {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let pairs = &self.pairs.iter().collect::<Vec<_>>();
    tokens.extend(quote! { { #(#pairs),* } });
  }
}

/// Helper parser for a single `key: value` pair. Like in `Fields`, the `value` is optional, and if
/// omitted, looks up the `key` as an expression. In the final parsed result, the `value` field will
/// always be populated.
pub struct KVPair {
  pub key: Ident,
  pub value: Expr,
  #[allow(unused)]
  pub colon: Option<syn::Token![:]>,
}

impl Parse for KVPair {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let key: Ident = input.parse()?;
    if input.peek(syn::Token![:]) {
      let colon = Some(input.parse::<syn::Token![:]>()?);
      let value: Expr = input.parse()?;
      Ok(KVPair { key, value, colon })
    } else {
      Ok(KVPair {
        value: parse_quote! { #key },
        key,
        colon: None,
      })
    }
  }
}

impl ToTokens for KVPair {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let key = &self.key;
    let value = &self.value;
    tokens.extend(quote! { #key: #value });
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

impl Parse for Retrieve {
  fn parse(input: ParseStream) -> syn::Result<Self> {
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

impl Parse for Persist {
  fn parse(input: ParseStream) -> syn::Result<Self> {
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

impl Parse for UpState {
  fn parse(input: ParseStream) -> syn::Result<Self> {
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

      let kvs: KVPairs = input.parse()?;
      Ok(UpState::Map {
        map_name: store_name,
        item_name,
        kvs,
      })
    }
  }
}

/// Custom parser for enumerate macro (MapName[prefixes...], min..max, descending)
pub struct Enumerate {
  pub map_name: Ident,
  pub prefixes: Vec<Expr>,
  pub bounds: EnumerateBounds,
  pub order: Order,
  pub idx: Option<Ident>,
}

/// Custom parser for delete macro (MapName[item_name]), (StoreName)
pub enum Delete {
  Map {
    map_name: Ident,
    item_name: Expr,
  },
  Store {
    store_name: Ident,
  },
}

impl Parse for Delete {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let store_name: Ident = input.parse()?;
    if input.peek(syn::token::Bracket) {
      let content;
      syn::bracketed!(content in input);
      let item_name: Expr = content.parse()?;
      Ok(Delete::Map { map_name: store_name, item_name })
    } else {
      Ok(Delete::Store { store_name })
    }
  }
}

/// Custom parser for assert macro (condition, error)
pub struct Assert {
  pub condition: Expr,
  pub error: Expr,
}

impl Parse for Assert {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let condition: Expr = input.parse()?;
    input.parse::<syn::Token![,]>()?;
    let error: Expr = input.parse()?;

    Ok(Assert { condition, error })
  }
}

pub enum Order {
  Ascending,
  Descending,
}

#[derive(Default)]
pub struct EnumerateBounds {
  pub start: Option<Expr>,
  pub end: Option<Expr>,
  pub closed: bool,
}

impl Parse for Enumerate {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let map_name: Ident = input.parse()?;
    let mut bounds: EnumerateBounds = Default::default();
    let mut order = Order::Ascending;
    let mut index_field = None;

    // Check for index field syntax (map_name.index_field)
    if input.peek(syn::Token![.]) {
      input.parse::<syn::Token![.]>()?;
      index_field = Some(input.parse()?);
    }

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
          order = Order::Descending;
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
      idx: index_field,
    })
  }
}

pub struct MapIndex {
  pub indexes: Vec<Expr>,
}

impl Parse for MapIndex {
  fn parse(input: ParseStream) -> syn::Result<Self> {
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
  pub fn peek(input: ParseStream) -> bool {
    input.peek(syn::token::Bracket)
  }
}

pub struct Invoke {
  pub recipient: Expr,
  pub msg: Expr,
  pub funds: Expr,
}

impl Parse for Invoke {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let recipient: Expr = input.parse()?;
    input.parse::<syn::Token![,]>()?;
    let msg: Expr = input.parse()?;

    let mut funds: Expr = syn::parse2(quote! { vec![] })?;

    if input.peek(syn::Token![,]) {
      let ident: Ident = input.parse()?;
      input.parse::<syn::Token![=]>()?;
      match ident.to_string().as_str() {
        "funds" => {
          funds = input.parse()?;
        }
        _ => {
          return Err(syn::Error::new(ident.span(), "Unknown argument"));
        }
      }
    }

    Ok(Invoke { recipient, msg, funds })
  }
}

impl ToTokens for Invoke {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    let Invoke { recipient, msg, funds } = self;

    tokens.extend(quote! {
      ctx.invoke(#recipient, #msg, #funds)
    });
  }
}

/// Helper parser for `emit!` macro
pub struct EmitEvent {
  pub name: String,
  pub attrs: EventAttributes,
}

impl Parse for EmitEvent {
  fn parse(input: ParseStream) -> syn::Result<Self> {
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
      ctx.emit(solarsail::cw_std::Event::new(#name)#(#attrs)*)
    });
  }
}

pub struct EventAttributes {
  pub pairs: Vec<(String, Expr)>,
}

impl Parse for EventAttributes {
  fn parse(input: ParseStream) -> syn::Result<Self> {
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

fn peek_ident(input: ParseStream) -> syn::Result<Ident> {
  input.fork().parse::<Ident>()
}

/// Helper parser for the first step of the solarize macro:
/// Distinguishing between fn & impl blocks.
pub enum SolarizePrep {
  Fn(ItemFn),
  Impl(ItemImpl),
  Ty(SolarizeType),
}

impl Parse for SolarizePrep {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    // temporary ParseStream to make checking the next few tokens easier
    // we don't want to actually process these tokens b/c they will be parsed by other parsers
    let tmp = input.fork();

    // ignore attributes
    while tmp.peek(syn::Token![#]) && tmp.peek2(syn::token::Bracket) {
      tmp.parse::<syn::Token![#]>()?;
      let _content;
      syn::bracketed!(_content in tmp);
    }

    // ignore leading pub visibility
    if tmp.peek(syn::Token![pub]) {
      tmp.parse::<syn::Token![pub]>()?;
    }

    if tmp.peek(syn::Token![impl]) {
      let impl_def: ItemImpl = input.parse()?;
      Ok(SolarizePrep::Impl(impl_def))
    } else if tmp.peek(syn::Token![struct]) {
      let struct_def: ItemStruct = input.parse()?;
      Ok(SolarizePrep::Ty(SolarizeType::Struct(struct_def)))
    } else if tmp.peek(syn::Token![enum]) {
      let enum_def: ItemEnum = input.parse()?;
      Ok(SolarizePrep::Ty(SolarizeType::Enum(enum_def)))
    } else if tmp.peek(syn::Token![fn]) {
      let func: ItemFn = input.parse()?;
      Ok(SolarizePrep::Fn(func))
    } else {
      return Err(syn::Error::new(tmp.span(), "Expected `impl`, `struct`, `enum`, or `fn`"));
    }
  }
}

pub enum SolarizeType {
  Struct(ItemStruct),
  Enum(ItemEnum),
}

impl ToTokens for SolarizeType {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      SolarizeType::Struct(item) => {
        tokens.extend(quote! { #item })
      }
      SolarizeType::Enum(item) => {
        tokens.extend(quote! { #item })
      }
    }
  }
}

/// Helper parser for attribute parameters.
pub struct SerdeParams {
  pub params: Punctuated<SerdeParam, Token![,]>,
}

impl Parse for SerdeParams {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let params = Punctuated::<SerdeParam, Token![,]>::parse_terminated(input)?;
    Ok(SerdeParams { params })
  }
}

impl ToTokens for SerdeParams {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    self.params.to_tokens(tokens);
  }
}

impl SerdeParams {
  #[allow(unused)]
  pub fn push_kw(&mut self, name: impl Into<Ident>) {
    self.params.push(SerdeParam::Keyword(name.into()));
  }

  pub fn push_kv(&mut self, name: impl Into<Ident>, value: Expr) {
    self.params.push(SerdeParam::KeyValue(name.into(), value));
  }

  /// Remove duplicate parameters from the list, keeping the last occurrence of each name.
  pub fn dedup(&mut self) {
    let mut kept_params = Vec::new();

    // TODO: potentially need to handle conflicting keys across types,
    // e.g. `rename(serialize = "ser_name")` vs `rename_all = "snake_case"`
    for (i, param) in self.params.iter().enumerate() {
      let current_name = param.name();
      let mut should_keep = true;

      for j in (i + 1)..self.params.len() {
        if self.params[j].name().to_string() == current_name.to_string() {
          should_keep = false;
          break;
        }
      }

      if should_keep {
        kept_params.push(param.clone());
      }
    }

    self.params = kept_params.into_iter().collect();
  }
}

/// Helper parser for the serde attribute parameters.
#[derive(Clone)]
pub enum SerdeParam {
  /// This parameter is a lone keyword, e.g. `untagged`.
  Keyword(Ident),
  /// This parameter uses a key-value pair syntax, e.g. `rename = "ser_name"`.
  KeyValue(Ident, Expr),
  /// This parameter uses function call syntax, e.g. `rename(serialize = "ser_name")`.
  /// As we're not interested in these, we don't touch the contained tokens.
  Function(Ident, TokenStream),
}

impl Parse for SerdeParam {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let name: Ident = if input.peek(syn::Token![crate]) {
      input.parse::<syn::Token![crate]>()?;
      make_ident("crate")
    } else {
      input.parse()?
    };

    if input.peek(syn::token::Paren) {
      let content;
      syn::parenthesized!(content in input);
      Ok(SerdeParam::Function(name, content.parse()?))
    } else if input.peek(syn::Token![=]) {
      input.parse::<syn::Token![=]>()?;
      let value: Expr = input.parse()?;
      Ok(SerdeParam::KeyValue(name, value))
    } else {
      Ok(SerdeParam::Keyword(name))
    }
  }
}

impl ToTokens for SerdeParam {
  fn to_tokens(&self, tokens: &mut TokenStream) {
    match self {
      SerdeParam::Keyword(name) => {
        tokens.extend(quote! { #name })
      }
      SerdeParam::KeyValue(name, value) => {
        tokens.extend(quote! { #name = #value })
      }
      SerdeParam::Function(name, content) => {
        tokens.extend(quote! { #name(#content) })
      }
    }
  }
}

impl SerdeParam {
  pub fn name(&self) -> &Ident {
    match self {
      SerdeParam::Keyword(name) => name,
      SerdeParam::KeyValue(name, _) => name,
      SerdeParam::Function(name, _) => name,
    }
  }
}

pub struct AuthorityArgs {
  pub authorities: Punctuated<Ident, syn::Token![,]>,
}

impl Parse for AuthorityArgs {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let authorities = input.parse_terminated(Ident::parse, syn::Token![,])?;
    Ok(AuthorityArgs { authorities })
  }
}
