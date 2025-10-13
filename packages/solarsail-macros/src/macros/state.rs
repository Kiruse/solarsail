use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

use crate::Order;
use crate::parsers::{Enumerate, Persist, Retrieve, State, StateMap, StateMapIndex, UpState};

pub type StateMacroInput = State;

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

  let ty_name = input.identifier
    .as_ref()
    .map(|ident| ident.clone())
    .unwrap_or_else(|| Ident::new("State", Span::call_site()));
  let ty_name = struct_name(&ty_name);

  let fn_name = input.identifier
    .as_ref()
    .map(|ident| ident.clone())
    .unwrap_or_else(|| Ident::new("STATE", Span::call_site()));
  let fn_name = storage_name(&fn_name);

  quote! {
    #[cosmwasm_schema::cw_serde]
    pub struct #ty_name {
      #(#field_definitions)*
    }

    pub fn #fn_name() -> ::cw_storage_plus::Item<#ty_name> {
      ::cw_storage_plus::Item::new("state")
    }
  }
}

pub fn state_map(input: &StateMap) -> TokenStream {
  let name = &input.name;
  let key_type = &input.key_type;
  let value_type = &input.value_type;

  let fn_name = storage_name(name);

  let key = name.to_string();

  // without indexes
  if input.indexes.is_empty() {
    quote! {
      pub fn #fn_name() -> ::cw_storage_plus::Map<#key_type, #value_type> {
        ::cw_storage_plus::Map::new(#key)
      }
    }
  }
  // with indexes
  else {
    let idx_fields = input.indexes
      .iter()
      .map(|StateMapIndex { field, ty, unique }| {
        if *unique {
          quote! {
            pub #field: ::cw_storage_plus::UniqueIndex<'a, #ty, #value_type, ()>
          }
        } else {
          quote! {
            pub #field: ::cw_storage_plus::MultiIndex<'a, #ty, #value_type, #key_type>
          }
        }
      })
      .collect::<Vec<_>>();

    let idx_init = input.indexes
      .iter()
      .map(|StateMapIndex { field, unique, .. }| {
        let idx_key = format!("{}__{}", key, field);
        if *unique {
          quote! {
            #field: ::cw_storage_plus::UniqueIndex::new(
              |data| data.#field.clone(),
              #idx_key
            )
          }
        } else {
          quote! {
            #field: ::cw_storage_plus::MultiIndex::new(
              |_pk, data| data.#field.clone(),
              #key,
              #idx_key
            )
          }
        }
      })
      .collect::<Vec<_>>();

    let idx_extract = input.indexes
      .iter()
      .map(|StateMapIndex { field, .. }| {
        quote! { &self.#field }
      })
      .collect::<Vec<_>>();

    let indexes_struct = Ident::new(
      &format!("{}Indexes", struct_name(name).to_string().to_case(Case::Pascal)),
      Span::mixed_site(),
    );

    let indexes_fn = Ident::new(
      &format!("{}_indexes", fn_name.to_string().to_case(Case::Snake)),
      Span::mixed_site(),
    );

    quote! {
      pub fn #fn_name<'a>() -> ::cw_storage_plus::IndexedMap<#key_type, #value_type, #indexes_struct<'a>> {
        ::cw_storage_plus::IndexedMap::new(#key, #indexes_fn())
      }

      pub struct #indexes_struct<'a> {
        #(#idx_fields),*
      }

      impl<'a> ::cw_storage_plus::IndexList<#value_type> for #indexes_struct<'a> {
        fn get_indexes(&self) -> Box<dyn Iterator<Item = &dyn ::cw_storage_plus::Index<#value_type>> + '_> {
          let v: Vec<&dyn ::cw_storage_plus::Index<#value_type>> = vec![
            #(#idx_extract),*
          ];
          Box::new(v.into_iter())
        }
      }

      fn #indexes_fn<'a>() -> #indexes_struct<'a> {
        #indexes_struct { #(#idx_init),* }
      }

      pub trait IndexedMapExt<'a, K, V, I> {
        fn idx(&self) -> #indexes_struct<'a>;
      }

      impl<'a, K, V> IndexedMapExt<'a, K, V, #indexes_struct<'a>> for ::cw_storage_plus::IndexedMap<K, V, #indexes_struct<'a>> {
        fn idx(&self) -> #indexes_struct<'a> {
          #indexes_fn()
        }
      }
    }
  }
}

pub fn retrieve(input: &Retrieve) -> TokenStream {
  match input {
    Retrieve::Map { map_name, item_name } => {
      let store_name = storage_name(map_name);
      quote! {
        #store_name().load(ctx.deps.storage, #item_name)
      }.into()
    }
    Retrieve::State { store_name } => {
      let store_name = storage_name(store_name);
      quote! {
        #store_name().load(ctx.deps.storage)
      }.into()
    }
  }
}

pub fn persist(input: &Persist) -> TokenStream {
  match input {
    Persist::Map { map_name, item_name, value } => {
      let store_name = storage_name(map_name);
      quote! {
        #store_name().save(ctx.deps.storage, #item_name, &#value)
      }.into()
    }
    Persist::State { store_name, value } |
    Persist::StructConstruction { store_name, value } => {
      let store_name = storage_name(store_name);
      quote! {
        #store_name().save(ctx.deps.storage, &#value)
      }.into()
    }
  }
}

pub fn upstate(input: &UpState) -> TokenStream {
  match input {
    UpState::Map { map_name, item_name, kvs } => {
      let store_name = storage_name(map_name);
      let pairs = kvs.pairs.iter().map(|(key, value)| {
        quote! { #key: #value }
      }).collect::<Vec<_>>();
      quote! {
        #store_name().update(ctx.deps.storage, #item_name, |old| -> Result<_, cosmwasm_std::StdError> {
          Ok(Item {
            #(#pairs,)*
            ..old
          })
        })
      }.into()
    }
    UpState::Store { store_name, kvs } => {
      let struct_name = struct_name(store_name);
      let store_name = storage_name(store_name);
      let pairs = kvs.pairs.iter().map(|(key, value)| {
        quote! { #key: #value }
      }).collect::<Vec<_>>();
      quote! {
        #store_name().update(ctx.deps.storage, |old| -> Result<_, cosmwasm_std::StdError> {
          Ok(#struct_name {
            #(#pairs,)*
            ..old
          })
        })
      }.into()
    }
  }
}

pub fn enumerate(input: &Enumerate) -> TokenStream {
  let Enumerate {
    map_name,
    prefixes,
    bounds,
    order,
    idx,
  } = input;

  let store_name = storage_name(map_name);

  let mut res = quote! { #store_name() };

  if let Some(idx) = idx {
    res = quote! { #res.idx().#idx };
  }

  if !prefixes.is_empty() {
    res = quote! { #res.prefix((#(#prefixes),*)) };
  }

  // Order type doesn't implement ToTokens, so we just manually wrap it
  let order = match order {
    Order::Ascending => quote! { ::cosmwasm_std::Order::Ascending },
    Order::Descending => quote! { ::cosmwasm_std::Order::Descending },
  };

  let min = match &bounds.start {
    None => quote! { None },
    Some(start) => quote! { #start.map(|v| ::cw_storage_plus::Bound::inclusive(v)) },
  };

  let max = match &bounds.end {
    None => quote! { None },
    Some(end) if bounds.closed => quote! { #end.map(|v| ::cw_storage_plus::Bound::inclusive(v)) },
    Some(end) => quote! { #end.map(|v| ::cw_storage_plus::Bound::exclusive(v)) },
  };

  quote! {
    #res.range(ctx.deps.storage, #min, #max, #order)
  }
}

fn storage_name(name: &Ident) -> Ident {
  Ident::new(
    &format!("storage_{}", name.to_string().to_case(Case::Snake)),
    name.span(),
  )
}

fn struct_name(name: &Ident) -> Ident {
  Ident::new(
    &format!("{}", name.to_string().to_case(Case::Pascal)),
    name.span(),
  )
}
