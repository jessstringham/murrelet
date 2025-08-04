extern crate proc_macro;

use darling::FromDeriveInput;
use derive_schema::FieldTokensSchema;
use parser::{GenFinal, LivecodeReceiver};
use proc_macro::TokenStream;

mod derive_schema;
mod parser;

#[proc_macro_derive(MurreletSchema, attributes(murrelet_schema))]
pub fn murrelet_livecode_derive_murrelet_gui(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    let ast_receiver = LivecodeReceiver::from_derive_input(&ast).unwrap();
    FieldTokensSchema::from_ast(ast_receiver).into()
}
