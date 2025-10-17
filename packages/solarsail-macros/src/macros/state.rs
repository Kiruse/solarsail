use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::Order;
use crate::macros::utils::rename_ident;
use crate::parsers::{Delete, Enumerate, Persist, Retrieve, StateIntegrated, StateMap, StateMapIndex, StateStore, UpState};

pub fn state_integrated(input: &StateIntegrated) -> TokenStream {
  let name = &input.name;
  let ty = &input.ty;
  let fn_name = storage_name(name);
  quote! {
    pub fn #fn_name() -> ::solarsail::cw_storage_plus::Item<#ty> {
      ::solarsail::cw_storage_plus::Item::new("state")
    }
  }
}

pub fn state_store(input: &StateStore) -> TokenStream {
  let (field_names, field_types): (Vec<_>, Vec<_>) = input.fields.named
    .iter()
    .map(|field| (field.ident.clone(), field.ty.clone()))
    .unzip();

  let ty_name = struct_name(&input.name);
  let fn_name = storage_name(&input.name);

  quote! {
    #[::solarsail::solarize]
    pub struct #ty_name {
      #(pub #field_names: #field_types),*
    }

    pub fn #fn_name() -> ::solarsail::cw_storage_plus::Item<#ty_name> {
      ::solarsail::cw_storage_plus::Item::new("state")
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
      pub fn #fn_name() -> ::solarsail::cw_storage_plus::Map<#key_type, #value_type> {
        ::solarsail::cw_storage_plus::Map::new(#key)
      }
    }
  }
  // with indexes
  else {
    let idx_init = input.indexes
      .iter()
      .map(|StateMapIndex { field, unique, .. }| {
        let idx_key = format!("{}__{}", key, field);
        if unique.is_some() {
          quote! {
            #field: ::solarsail::cw_storage_plus::UniqueIndex::new(
              |data| data.#field.clone(),
              #idx_key
            )
          }
        } else {
          quote! {
            #field: ::solarsail::cw_storage_plus::MultiIndex::new(
              |_pk, data| data.#field.clone(),
              #key,
              #idx_key
            )
          }
        }
      })
      .collect::<Vec<_>>();

    let (fields, field_types): (Vec<_>, Vec<_>) = input.indexes
      .iter()
      .map(|StateMapIndex { field, ty, unique }| {
        let ty = if unique.is_some() {
          quote! { ::solarsail::cw_storage_plus::UniqueIndex<'a, #ty, #value_type, ()> }
        } else {
          quote! { ::solarsail::cw_storage_plus::MultiIndex<'a, #ty, #value_type, #key_type> }
        };
        (field, ty)
      })
      .unzip();

    let indexes_struct = rename_ident!(Case::Pascal, "{}Indexes", name);
    let indexes_fn = rename_ident!(Case::Snake, "{}_indexes", fn_name);

    quote! {
      pub fn #fn_name<'a>() -> ::solarsail::cw_storage_plus::IndexedMap<#key_type, #value_type, #indexes_struct<'a>> {
        ::solarsail::cw_storage_plus::IndexedMap::new(#key, #indexes_fn())
      }

      pub struct #indexes_struct<'a> {
        #(pub #fields: #field_types),*
      }

      impl<'a> ::solarsail::cw_storage_plus::IndexList<#value_type> for #indexes_struct<'a> {
        fn get_indexes(&self) -> Box<dyn Iterator<Item = &dyn ::solarsail::cw_storage_plus::Index<#value_type>> + '_> {
          let v: Vec<&dyn ::solarsail::cw_storage_plus::Index<#value_type>> = vec![
            #(&self.#fields),*
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

      impl<'a, K, V> IndexedMapExt<'a, K, V, #indexes_struct<'a>> for ::solarsail::cw_storage_plus::IndexedMap<K, V, #indexes_struct<'a>> {
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
      let (keys, values): (Vec<_>, Vec<_>) = kvs.pairs
        .iter()
        .map(|pair| (&pair.key, &pair.value))
        .unzip();
      quote! {
        #store_name().update(ctx.deps.storage, #item_name, |old| -> Result<_, ::solarsail::cw_std::StdError> {
          let mut old = old.ok_or(::solarsail::cw_std::StdError::msg("old state not found"))?;
          #(old.#keys = #values;)*
          Ok(old)
        })
      }.into()
    }
    UpState::Store { store_name, kvs } => {
      let store_name = storage_name(store_name);
      let (keys, values): (Vec<_>, Vec<_>) = kvs.pairs
        .iter()
        .map(|pair| (&pair.key, &pair.value))
        .unzip();
      quote! {
        #store_name().update(ctx.deps.storage, |mut old| -> Result<_, ::solarsail::cw_std::StdError> {
          #(old.#keys = #values;)*
          Ok(old)
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
    Order::Ascending => quote! { ::solarsail::cw_std::Order::Ascending },
    Order::Descending => quote! { ::solarsail::cw_std::Order::Descending },
  };

  let min = match &bounds.start {
    None => quote! { None },
    Some(start) => quote! { #start.map(|v| ::solarsail::cw_storage_plus::Bound::inclusive(v)) },
  };

  let max = match &bounds.end {
    None => quote! { None },
    Some(end) if bounds.closed => quote! { #end.map(|v| ::solarsail::cw_storage_plus::Bound::inclusive(v)) },
    Some(end) => quote! { #end.map(|v| ::solarsail::cw_storage_plus::Bound::exclusive(v)) },
  };

  quote! {
    #res.range(ctx.deps.storage, #min, #max, #order)
  }
}

pub fn delete(input: &Delete) -> TokenStream {
  match input {
    Delete::Map { map_name, item_name } => {
      let store_name = storage_name(map_name);
      quote! { #store_name().remove(ctx.deps.storage, #item_name) }.into()
    }
    Delete::Store { store_name } => {
      let store_name = storage_name(store_name);
      quote! { #store_name().remove(ctx.deps.storage) }.into()
    }
  }
}

fn storage_name(name: &Ident) -> Ident {
  rename_ident!(Case::Snake, "storage_{}", name)
}

fn struct_name(name: &Ident) -> Ident {
  rename_ident!(Case::Pascal, "{}", name)
}
