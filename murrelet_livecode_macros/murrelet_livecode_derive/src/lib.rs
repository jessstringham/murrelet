// This is where I dump all of my proc macros.
// This is very hacky and incomplete, and I'm adding things as I need them. That said,
// I can usually make a lot of things work just with this  (e.g. if I need a fixed-size
// array, I'll just use a Vec and then convert it).

extern crate proc_macro;

mod derive_boop;
mod derive_livecode;
mod derive_nestedit;
mod derive_lazy;
mod derive_unitcell;
mod parser;
mod toplevel;

use darling::FromDeriveInput;
use derive_boop::FieldTokensBoop;
use derive_lazy::FieldTokensLazy;
use derive_livecode::FieldTokensLivecode;
use derive_nestedit::FieldTokensNestEdit;
use derive_unitcell::FieldTokensUnitCell;
use parser::{GenFinal, LivecodeReceiver};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::parse_macro_input;
use toplevel::{impl_all_the_traits, top_level_livecode};

use quote::quote;

fn livecode_parse_ast(rec: LivecodeReceiver) -> TokenStream2 {
    FieldTokensLivecode::from_ast(rec)
}

fn unitcell_parse_ast(rec: LivecodeReceiver) -> TokenStream2 {
    FieldTokensUnitCell::from_ast(rec)
}

fn lazy_parse_ast(rec: LivecodeReceiver) -> TokenStream2 {
    FieldTokensLazy::from_ast(rec)
}

fn boop_parse_ast(rec: LivecodeReceiver) -> TokenStream2 {
    FieldTokensBoop::from_ast(rec)
}

fn nestedit_parse_ast(rec: LivecodeReceiver) -> TokenStream2 {
    FieldTokensNestEdit::from_ast(rec)
}

// meh, i usually need all of these, so throw them all in.
#[proc_macro_derive(Livecode, attributes(livecode))]
pub fn murrelet_livecode_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let ast_receiver = LivecodeReceiver::from_derive_input(&ast).unwrap();

    let livecode = livecode_parse_ast(ast_receiver.clone());
    let boop = boop_parse_ast(ast_receiver.clone());
    let nested = nestedit_parse_ast(ast_receiver.clone());

    quote!(
        #livecode
        #boop
        #nested
    )
    .into()
}

// eh, these two are useful for things that are UnitCells but not Livecode
#[proc_macro_derive(Lazy, attributes(livecode))]
pub fn murrelet_livecode_derive_lazy(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let ast_receiver = LivecodeReceiver::from_derive_input(&ast).unwrap();
    lazy_parse_ast(ast_receiver.clone()).into()
}

// eh, these two are useful for things that are UnitCells but not Livecode
#[proc_macro_derive(Boop, attributes(livecode))]
pub fn murrelet_livecode_derive_boop(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let ast_receiver = LivecodeReceiver::from_derive_input(&ast).unwrap();
    boop_parse_ast(ast_receiver.clone()).into()
}

#[proc_macro_derive(NestEdit, attributes(livecode))]
pub fn murrelet_livecode_derive_nestedit(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let ast_receiver = LivecodeReceiver::from_derive_input(&ast).unwrap();
    nestedit_parse_ast(ast_receiver.clone()).into()
}

#[proc_macro_derive(UnitCell, attributes(livecode))]
pub fn murrelet_livecode_derive_unitcell(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let ast_receiver = LivecodeReceiver::from_derive_input(&ast).unwrap();
    unitcell_parse_ast(ast_receiver.clone()).into()
}

// todo, this is if we need to load config
#[proc_macro_derive(TopLevelLiveCode, attributes(livecode))]
pub fn murrelet_livecode_top_level_livecode(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    top_level_livecode(ast.ident).into()
}

#[proc_macro_derive(LiveCoderTrait, attributes(livecode))]
pub fn murrelet_livecode_livecoder_traits(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    impl_all_the_traits(ast.ident).into()
}
