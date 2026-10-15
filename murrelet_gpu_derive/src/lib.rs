extern crate proc_macro;

mod derive_graphics_trait;

use derive_graphics_trait::impl_graphics_trait;
use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro_derive(LiveGraphics, attributes(graphics))]
pub fn murrelet_livecode_graphics(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    impl_graphics_trait(ast).into()
}
