mod toplevel;

extern crate proc_macro;

use proc_macro::TokenStream;
use syn::parse_macro_input;
use crate::toplevel::{impl_all_the_traits, top_level_livecode, top_level_livecode_json};

// todo, this is if we need to load config
#[proc_macro_derive(TopLevelLiveCode, attributes(livecode))]
pub fn murrelet_livecode_top_level_livecode(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    top_level_livecode(ast.ident).into()
}

#[proc_macro_derive(TopLevelLiveCodeJson, attributes(livecode))]
pub fn murrelet_livecode_top_level_livecode_json(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    top_level_livecode_json(ast.ident).into()
}

#[proc_macro_derive(LiveCoderTrait, attributes(livecode))]
pub fn murrelet_livecode_livecoder_traits(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    impl_all_the_traits(ast.ident).into()
}
