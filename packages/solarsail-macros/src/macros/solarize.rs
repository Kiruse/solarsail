use crate::macros::contract::{ContractNames, MsgKind};
use crate::macros::returns::transform_returns;
use crate::macros::utils::{MsgParams, extract_msg_meta, extract_msg_params, generate_msg_extractions, generate_struct_fields, make_ident, rename_ident};
use crate::parsers::{SerdeParams, SolarizeType};
use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Expr, FnArg, Ident, ImplItem, ImplItemFn, ItemImpl, Meta, MetaList, Pat, PatType, Path, Token, Type, TypePath, Visibility, parse_quote};
use syn::spanned::Spanned;

pub struct SolarizeImplArgs {
  pub kind: MsgKind,
  pub generate_entrypoints: bool,
}

impl Parse for SolarizeImplArgs {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let kind: Ident = input.parse()?;
    let kind = MsgKind::try_from(kind)?;

    let mut generate_entrypoints = true;

    while !input.is_empty() {
      input.parse::<Token![,]>()?;
      let key: Ident = input.parse()?;

      // key/value pair
      if input.peek(Token![=]) {
        input.parse::<Token![=]>()?;
        let _value: Expr = input.parse()?;
        match key.to_string().as_str() {
          _ => return Err(syn::Error::new(key.span(), "Invalid solarize impl argument")),
        }
      }
      // key only
      else {
        match key.to_string().as_str() {
          "no_entrypoints" => {
            generate_entrypoints = false;
          }
          _ => return Err(syn::Error::new(key.span(), "Invalid solarize impl argument")),
        }
      }
    }

    Ok(SolarizeImplArgs { kind, generate_entrypoints })
  }
}

/// Transform a `#[solarize(execute|query)] impl` block into a proper contract implementation.
pub fn transform_impl(args: SolarizeImplArgs, mut input: ItemImpl) -> Result<TokenStream, syn::Error> {
  let SolarizeImplArgs {
    kind,
    generate_entrypoints,
  } = args;

  let contract_name = contract_name(&input)?;
  let names = ContractNames::new(contract_name, kind);
  let mut msgs_pub: Vec<_> = vec![];
  let mut msgs_int: Vec<_> = vec![];
  let mut msg_structs = vec![];

  for item in input.items.iter_mut() {
    match item {
      ImplItem::Fn(func) => {
        if matches!(names.kind(), MsgKind::Query) {
          transform_returns(func, &mut msg_structs)?;
        }
        transform_impl_fn(func, &mut msg_structs)?;
        if matches!(func.vis, Visibility::Public(_)) {
          msgs_pub.push(func.clone());
        } else {
          msgs_int.push(func.clone());
        }
      }
      _ => {}
    }
  }

  let entrypoint = if generate_entrypoints {
    let msgs = msgs_pub.iter().chain(msgs_int.iter()).map(|f| f.sig.ident.clone()).collect::<Vec<_>>();
    generate_entrypoint(&names, &msgs)
  } else {
    quote! {}
  };

  let impl_code = generate_impl(&names, &msgs_pub, &msgs_int);

  Ok(quote! {
    #(#msg_structs)*
    #impl_code
    #entrypoint
  })
}

/// Transform an impl function into a proper message handler.
fn transform_impl_fn(func: &mut ImplItemFn, msgs: &mut Vec<TokenStream>) -> Result<(), syn::Error> {
  let msg = make_ident("msg");
  let params = extract_msg_params(&func.sig)?;
  let msg_struct_name = rename_ident!(Case::Pascal, "{}Msg", func.sig.ident);

  match params {
    MsgParams::Fields(fields) => {
      let struct_fields = generate_struct_fields(&fields);

      let block = generate_msg_extractions(&fields, msg.clone());
      func.block.stmts.splice(0..0, block.stmts);

      msgs.push(quote! {
        #[solarsail::solarize]
        pub struct #msg_struct_name {
          #(#struct_fields)*
        }
      });
    }
    MsgParams::Msg(msg_arg) => {
      let (ty, block) = extract_msg_meta(msg.clone(), msg_arg)?;
      func.block.stmts.splice(0..0, block.stmts);
      msgs.push(quote! { type #msg_struct_name = #ty; });
    }
  }

  // transform function signature
  let mut args = func.sig.inputs.iter().cloned().collect::<Vec<_>>();
  // retain only self & ctx parameters
  args.retain(|arg| match arg {
    FnArg::Receiver(_) => true,
    FnArg::Typed(PatType { pat, .. }) => {
      match &**pat {
        Pat::Ident(pat_ident) => pat_ident.ident == "ctx" || pat_ident.ident == "_ctx",
        _ => false,
      }
    }
  });
  // append a msg parameter
  args.push(parse_quote! { #msg: #msg_struct_name });
  // update signature inputs
  func.sig.inputs = args.into_iter().collect();

  Ok(())
}

/// Transform a `#[solarize] struct|enum` attribute into a proper message type.
pub fn transform_ty(mut ty: SolarizeType) -> syn::Result<TokenStream> {
  match &mut ty {
    SolarizeType::Struct(item) =>
      transform_ty_attrs(&mut item.attrs, false)?,
    SolarizeType::Enum(item) =>
      transform_ty_attrs(&mut item.attrs, true)?,
  };

  Ok(ty.to_token_stream())
}

/// Transform attributes of a `#[solarize] struct|enum` to add proper support for message types /
/// network de/serialization.
fn transform_ty_attrs(attrs: &mut Vec<Attribute>, is_enum: bool) -> syn::Result<()> {
  let serde_attr = attrs
    .iter_mut()
    .find(|attr| match &attr.meta {
      Meta::Path(path) => path.is_ident("serde"),
      Meta::List(list) => list.path.is_ident("serde"),
      _ => false,
    });

  let mut params: SerdeParams = parse_quote! {
    crate = "solarsail::serde"
  };

  if is_enum {
    params.push_kv(
      make_ident("rename_all"),
      parse_quote! { "snake_case" },
    );
  }

  match serde_attr {
    // if the attribute exists, alter it in-place
    Some(attr) => {
      match &mut attr.meta {
        // if it's a list, extend the existing parameters
        Meta::List(meta) => {
          let existing_params: SerdeParams = syn::parse2(meta.tokens.clone())?;
          params.params.extend(existing_params.params);
          params.dedup();
          meta.tokens = params.to_token_stream();
        }
        // if it's a single path value, wrap it in a list and add our parameters to it
        Meta::Path(_) => {
          attr.meta = Meta::List(MetaList {
            delimiter: syn::MacroDelimiter::Paren(syn::token::Paren(Span::mixed_site())),
            path: Path::from(make_ident("serde")),
            tokens: params.to_token_stream(),
          });
        }
        _ => unreachable!()
      }
    }
    // if the attribute doesn't exist, add it with our parameters
    None => {
      let attr = parse_quote!(#[serde(#params)]);
      attrs.push(attr);
    }
  };

  // add the necessary attributes for proper de/serialization & schema generation
  attrs.insert(0, parse_quote! {
    #[derive(
      solarsail::serde::Serialize,
      solarsail::serde::Deserialize,
      ::std::clone::Clone,
      ::std::fmt::Debug,
      ::std::cmp::PartialEq,
      solarsail::schemars::JsonSchema,
      solarsail::cosmwasm_schema::cw_schema::Schemaifier,
    )]
  });

  attrs.push(parse_quote! {
    #[allow(clippy::derive_partial_eq_without_eq)]
  });

  attrs.push(parse_quote! {
    #[schemaifier(crate = "solarsail::cosmwasm_schema::cw_schema")]
  });

  attrs.push(parse_quote! {
    #[schemars(crate = "solarsail::schemars")]
  });

  Ok(())
}

/// Generate the main query entrypoint function and QueryMsg enum
fn generate_entrypoint(names: &ContractNames, fns: &[Ident]) -> TokenStream {
  if fns.is_empty() {
    return quote! {};
  }

  let enum_name = names.msg_entry();

  match names.kind() {
    MsgKind::Execute => quote! {
      #[cfg_attr(not(feature = "library"), solarsail::cw_std::entry_point)]
      pub fn execute(
        deps: solarsail::cw_std::DepsMut,
        env: solarsail::cw_std::Env,
        info: solarsail::cw_std::MessageInfo,
        msg: #enum_name,
      ) -> solarsail::scaffold::ExecuteResult<ContractError> {
        let mut ctx = solarsail::ExecuteContext::new(deps, env, info);
        let res = msg.handle(&mut ctx)?;
        Ok(solarsail::scaffold::ExecuteResponse::new()
          .add_submessages(ctx.submsgs)
          .add_events(ctx.events))
      }
    },
    MsgKind::Query => quote! {
      #[cfg_attr(not(feature = "library"), solarsail::cw_std::entry_point)]
      pub fn query(
        deps: solarsail::cw_std::Deps,
        env: solarsail::cw_std::Env,
        msg: #enum_name,
      ) -> solarsail::scaffold::QueryResult<solarsail::cw_std::StdError> {
        msg.handle(solarsail::QueryContext::new(deps, env))
          .map_err(|e| solarsail::cw_std::StdError::msg(e.to_string()))
      }
    }
  }
}

/// Generate the code of a `#[solarize(execute|query)] impl` block.
fn generate_impl(names: &ContractNames, fns_pub: &[ImplItemFn], fns_int: &[ImplItemFn]) -> TokenStream {
  let enum_name = names.msg_internal();
  let contract_name = names.full_name();

  let ctx_ty = names.ctx_ty();
  let ctx_ty = match names.kind() {
    MsgKind::Execute => quote! { &mut solarsail::scaffold::#ctx_ty },
    MsgKind::Query => quote! { solarsail::scaffold::#ctx_ty },
  };

  let scaffold_trait = names.scaffold_trait();
  let trait_base = names.trait_base();
  let trait_public = names.trait_public();
  let trait_internal = names.trait_internal();

  let fn_names_pub: Vec<_> = fns_pub.iter().map(|f| &f.sig.ident).collect();
  let fn_names_int: Vec<_> = fns_int.iter().map(|f| &f.sig.ident).collect();
  let fns_pub_msg: Vec<_> = fns_pub.iter().map(|f| rename_ident!(Case::Pascal, "{}Msg", f.sig.ident)).collect();
  let fns_int_msg: Vec<_> = fns_int.iter().map(|f| rename_ident!(Case::Pascal, "{}Msg", f.sig.ident)).collect();
  let result_tys_pub: Vec<_> = fns_pub.iter().map(|f| &f.sig.output).collect();
  let result_tys_int: Vec<_> = fns_int.iter().map(|f| &f.sig.output).collect();

  let fns: Vec<Ident> = fns_pub
    .iter()
    .chain(fns_int.iter())
    .map(|f| f.sig.ident.clone())
    .collect();
  let variants: Vec<Ident> = fns.iter().map(|f| rename_ident!(Case::Pascal, "{}", f)).collect();
  let msgs: Vec<Ident> = fns.iter().map(|f| rename_ident!(Case::Pascal, "{}Msg", f)).collect();

  let handler = match names.kind() {
    MsgKind::Execute => quote! {
      fn handle(&self, ctx: #ctx_ty) -> Result<(), ContractError> {
        match self {
          #(Self::#variants(msg) => #contract_name::#fns(ctx, msg.clone())),*
        }
      }
    },
    MsgKind::Query => quote! {
      fn handle(&self, ctx: #ctx_ty) -> solarsail::scaffold::QueryResult<ContractError> {
        match self {
          #(Self::#variants(msg) => Ok(solarsail::cw_std::to_json_binary(&#contract_name::#fns(ctx, msg.clone())?)?)),*
        }
      }
    },
  };

  let fns_pub = fns_pub
    .iter()
    .map(|f| {
      let mut res = f.clone();
      res.vis = Visibility::Inherited;
      res
    })
    .collect::<Vec<_>>();
  let fns_int = fns_int
    .iter()
    .map(|f| {
      let mut res = f.clone();
      res.vis = Visibility::Inherited;
      res
    })
    .collect::<Vec<_>>();

  quote! {
    /// Public interface of this contract. Delegates to the original contract implementation.
    pub trait #trait_public {
      #(fn #fn_names_pub(
        ctx: #ctx_ty,
        msg: #fns_pub_msg,
      ) #result_tys_pub {
        #contract_name::#fn_names_pub(ctx, msg)
      })*
    }

    /// Full public & internal interface of this contract. Delegates to the original contract implementation.
    pub trait #trait_internal: #trait_public + #trait_base {
      #(fn #fn_names_int(
        ctx: #ctx_ty,
        msg: #fns_int_msg,
      ) #result_tys_int {
        #contract_name::#fn_names_int(ctx, msg)
      })*
    }

    #[solarsail::solarize]
    pub enum #enum_name {
      #(#variants(#msgs)),*
    }

    // Scaffold implementation which delegates to the specialized implementations
    impl #scaffold_trait for #enum_name {
      type Error = ContractError;
      #handler
    }

    // This is just a marker trait to force the developer to implement the parent traits
    impl #trait_base for #contract_name {}

    // Specialized internal implementation based on original definitions
    impl #trait_internal for #contract_name {
      #(#fns_int)*
    }

    // Specialized public implementation based on original definitions
    impl #trait_public for #contract_name {
      #(#fns_pub)*
    }
  }
}

/// Extract the name of the contract from the `impl` block, assuming the `name` in `impl <name>` is
/// the contract name & removing the `Contract` suffix.
fn contract_name(input: &ItemImpl) -> syn::Result<Ident> {
  let full = match &*input.self_ty {
    Type::Path(TypePath { path, .. }) => path.segments.last().unwrap().ident.clone(),
    _ => return Err(syn::Error::new(input.self_ty.span(), "Invalid contract name")),
  };
  Ok(Ident::new(
    &full.to_string().replace("Contract", ""),
    input.self_ty.span(),
  ))
}
