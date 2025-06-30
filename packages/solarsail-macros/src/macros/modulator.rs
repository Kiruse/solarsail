use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse::{Parse, ParseStream}, Block, Expr, Stmt, Visibility};

pub fn modulator(modul: &ItemModulator) -> TokenStream {
  let func_name = &modul.name;
  let func_vis = &Visibility::Public(Default::default());

  // Parse the function block to find the delimiter
  let (pre_statements, post_statements) = parse_modulator_block(&modul.body);

  // Generate pre function name
  let pre_func_name = syn::Ident::new(
    &format!("{}_pre", func_name),
    proc_macro2::Span::call_site(),
  );

  // Generate post function name
  let post_func_name = syn::Ident::new(
    &format!("{}_post", func_name),
    proc_macro2::Span::call_site(),
  );

  // Generate the pre function
  let pre_function = quote! {
    #func_vis fn #pre_func_name(ctx: &::solarsail::ExecuteContext) -> Result<(), ContractError> {
      #(#pre_statements)*
      Ok(())
    }
  };

  // Generate the post function
  let post_function = quote! {
    #func_vis fn #post_func_name(ctx: &::solarsail::ExecuteContext) -> Result<(), ContractError> {
      #(#post_statements)*
      Ok(())
    }
  };

  quote! {
    #pre_function
    #post_function
  }
}

fn parse_modulator_block(block: &Block) -> (Vec<&Stmt>, Vec<&Stmt>) {
  let mut pre_statements = Vec::new();
  let mut post_statements = Vec::new();
  let mut found_delimiter = false;

  for stmt in &block.stmts {
    if !found_delimiter {
      // Check if this statement is the delimiter `_;`
      if let Stmt::Expr(expr, Some(_)) = stmt {
        if let Expr::Path(expr_path) = expr {
          if expr_path.path.segments.len() == 1 && expr_path.path.segments[0].ident == "_" {
            found_delimiter = true;
            continue;
          }
        }
      }
      pre_statements.push(stmt);
    } else {
      post_statements.push(stmt);
    }
  }

  (pre_statements, post_statements)
}

pub struct ItemModulator {
  pub name: syn::Ident,
  pub body: syn::Block,
}

impl Parse for ItemModulator {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let name: syn::Ident = input.parse()?;
    let body: syn::Block = input.parse()?;
    Ok(Self { name, body })
  }
}
