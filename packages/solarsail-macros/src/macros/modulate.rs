use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, ItemFn};

pub fn modulate(func: &ItemFn, modulator: Ident) -> TokenStream {
  let func_name = &func.sig.ident;
  let func_vis = &func.vis;
  let func_block = &func.block;
  let func_attrs = &func.attrs;
  let func_args = &func.sig.inputs;
  let func_return = &func.sig.output;

  let pre_name = syn::Ident::new(
    &format!("{}_pre", modulator),
    proc_macro2::Span::call_site(),
  );
  let post_name = syn::Ident::new(
    &format!("{}_post", modulator),
    proc_macro2::Span::call_site(),
  );

  quote! {
    #(#func_attrs)*
    #func_vis fn #func_name(#func_args) #func_return {
      if let Err(err) = #pre_name(ctx) {
        return Err(err);
      }
      let result = #func_block;
      if let Err(err) = #post_name(ctx) {
        return Err(err);
      }
      result
    }
  }
}
