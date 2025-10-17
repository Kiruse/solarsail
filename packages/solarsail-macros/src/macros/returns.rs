use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{GenericArgument, Ident, ImplItemFn, Path, PathArguments, Stmt, Type, TypeArray, TypeGroup, TypeParen, TypePath, TypePtr, TypeReference, TypeSlice, TypeTuple, Visibility, parse_quote};
use syn::spanned::Spanned;

use crate::macros::utils::{find_attr, rename_ident};
use crate::parsers::ReturnsArgs;

/// Transform an impl function within a `#[solarize(query)]` block
pub fn transform_returns(func: &mut ImplItemFn, msgs: &mut Vec<TokenStream>) -> Result<(), syn::Error> {
  let args = match find_attr(&func.attrs, "returns") {
    Some(attr) => attr.parse_args::<ReturnsArgs>()?,
    None => return Ok(()), // bail if no `returns` attribute present
  };

  func.attrs.retain(|attr| !attr.path().is_ident("returns"));

  // generate the response struct
  let msg_name = rename_ident!(Case::Pascal, "{}Response", func.sig.ident);
  let mut msg_body = args.ty;
  for field in msg_body.named.iter_mut() {
    field.vis = Visibility::Public(Default::default());
  }
  msgs.push(quote! {
    #[::solarsail::solarize]
    pub struct #msg_name #msg_body
  });

  // find & replace the `_` infer placeholder in the return value declaration
  match &mut func.sig.output {
    syn::ReturnType::Default => Err(syn::Error::new(func.sig.ident.span(), "Query functions must return a value")),
    syn::ReturnType::Type(_, ty) => {
      if ty.is_infer() {
        func.sig.output = syn::ReturnType::Type(
          Default::default(),
          Box::new(make_type(&msg_name)),
        );
      } else {
        ty.replace_infer(&msg_name)?;
      }
      Ok(())
    }
  }?;

  // inject type alias for the `response!` macro
  let stmt: Stmt = parse_quote! { type SSAnonymousResponse = #msg_name; };
  func.block.stmts.insert(0, stmt);
  Ok(())
}

/// An assistant trait for replacing the `_` infer placeholder within compatible types.
pub trait InferReplacer {
  /// Whether this type is the `TypeInfer` token.
  fn is_infer(&self) -> bool { false }

  /// Recursively replace the `TypeInfer` token with the given message name within this syntax tree.
  fn replace_infer(&mut self, msg_name: &Ident) -> syn::Result<()>;
}

impl InferReplacer for Type {
  fn is_infer(&self) -> bool {
    matches!(self, Type::Infer(_))
  }

  fn replace_infer(&mut self, msg_name: &Ident) -> syn::Result<()> {
    match self {
      Type::Array(elem) => elem.replace_infer(msg_name),
      Type::Group(group) => group.replace_infer(msg_name),
      Type::Infer(_) => Err(syn::Error::new(self.span(), "Cannot replace infer token within an infer type")),
      Type::Never(_) => Ok(()),
      Type::Paren(paren) => paren.replace_infer(msg_name),
      Type::Path(path) => path.replace_infer(msg_name),
      Type::Ptr(ptr) => ptr.replace_infer(msg_name),
      Type::Reference(rf) => rf.replace_infer(msg_name),
      Type::Slice(slice) => slice.replace_infer(msg_name),
      Type::Tuple(tuple) => tuple.replace_infer(msg_name),
      _ => Err(syn::Error::new(self.span(), "Invalid infer return type")),
    }
  }
}

impl InferReplacer for TypeArray {
  fn replace_infer(&mut self, msg_name: &Ident) -> syn::Result<()> {
    if self.elem.is_infer() {
      *self.elem = make_type(msg_name);
    } else {
      self.elem.replace_infer(msg_name)?;
    }
    Ok(())
  }
}

impl InferReplacer for TypeGroup {
  fn replace_infer(&mut self, msg_name: &Ident) -> syn::Result<()> {
    if self.elem.is_infer() {
      *self.elem = make_type(msg_name);
    } else {
      self.elem.replace_infer(msg_name)?;
    }
    Ok(())
  }
}

impl InferReplacer for TypeParen {
  fn replace_infer(&mut self, msg_name: &Ident) -> syn::Result<()> {
    if self.elem.is_infer() {
      *self.elem = make_type(msg_name);
    } else {
      self.elem.replace_infer(msg_name)?;
    }
    Ok(())
  }
}

impl InferReplacer for TypePath {
  fn replace_infer(&mut self, msg_name: &Ident) -> syn::Result<()> {
    self.path.replace_infer(msg_name)
  }
}

impl InferReplacer for Path {
  fn replace_infer(&mut self, msg_name: &Ident) -> syn::Result<()> {
    for segment in self.segments.iter_mut() {
      match &mut segment.arguments {
        PathArguments::AngleBracketed(args) => {
          for arg in args.args.iter_mut() {
            if let GenericArgument::Type(ty) = arg {
              if ty.is_infer() {
                *arg = GenericArgument::Type(make_type(msg_name));
              } else {
                ty.replace_infer(msg_name)?;
              }
            }
          }
        }
        PathArguments::Parenthesized(args) => {
          for arg in args.inputs.iter_mut() {
            arg.replace_infer(msg_name)?;
          }
        }
        PathArguments::None => {}
      }
    }
    Ok(())
  }
}

impl InferReplacer for TypePtr {
  fn replace_infer(&mut self, msg_name: &Ident) -> syn::Result<()> {
    if self.elem.is_infer() {
      *self.elem = make_type(msg_name);
    } else {
      self.elem.replace_infer(msg_name)?;
    }
    Ok(())
  }
}

impl InferReplacer for TypeReference {
  fn replace_infer(&mut self, msg_name: &Ident) -> syn::Result<()> {
    if self.elem.is_infer() {
      *self.elem = make_type(msg_name);
    } else {
      self.elem.replace_infer(msg_name)?;
    }
    Ok(())
  }
}

impl InferReplacer for TypeSlice {
  fn replace_infer(&mut self, msg_name: &Ident) -> syn::Result<()> {
    if self.elem.is_infer() {
      *self.elem = make_type(msg_name);
    } else {
      self.elem.replace_infer(msg_name)?;
    }
    Ok(())
  }
}

impl InferReplacer for TypeTuple {
  fn replace_infer(&mut self, msg_name: &Ident) -> syn::Result<()> {
    for elem in self.elems.iter_mut() {
      *elem = make_type(msg_name);
    }
    Ok(())
  }
}

pub fn make_type(msg_name: &Ident) -> Type {
  Type::Path(TypePath {
    qself: None,
    path: Path::from(msg_name.clone()),
  })
}
