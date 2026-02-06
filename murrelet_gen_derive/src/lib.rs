extern crate proc_macro;

use darling::FromDeriveInput;
use derive_gen::FieldTokensGen;
use parser::{GenFinal, LivecodeReceiver};
use proc_macro::TokenStream;

mod derive_gen;
mod gen_methods;
mod parser;

use gen_methods::GenMethod;

#[proc_macro_derive(MurreletGen, attributes(murrelet_gen))]
pub fn murrelet_livecode_derive_murrelet_gen(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    let ast_receiver = LivecodeReceiver::from_derive_input(&ast).unwrap();
    FieldTokensGen::from_ast(ast_receiver).into()
}
