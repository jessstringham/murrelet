// This is where I dump all of my proc macros.
// This is very hacky and incomplete, and I'm adding things as I need them. That said,
// I can usually make a lot of things work just with this  (e.g. if I need a fixed-size
// array, I'll just use a Vec and then convert it).

extern crate proc_macro;

mod derive_boop;
mod derive_graphics_trait;
mod derive_lazy;
mod derive_lerpable;
mod derive_livecode;
mod derive_nestedit;
mod parser;
mod toplevel;

use darling::FromDeriveInput;
use derive_boop::FieldTokensBoop;
use derive_graphics_trait::impl_graphics_trait;
use derive_lazy::FieldTokensLazy;
use derive_lerpable::FieldTokensLerpable;
use derive_livecode::FieldTokensLivecode;
use derive_nestedit::FieldTokensNestEdit;
use parser::{GenFinal, LivecodeReceiver};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::parse_macro_input;
use toplevel::{impl_all_the_traits, top_level_livecode};

use quote::quote;

fn livecode_parse_ast(rec: LivecodeReceiver) -> TokenStream2 {
    FieldTokensLivecode::from_ast(rec)
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

fn lerpable_parse_ast(rec: LivecodeReceiver) -> TokenStream2 {
    FieldTokensLerpable::from_ast(rec)
}

// derives all of the macros I usually need
#[proc_macro_derive(Livecode, attributes(livecode))]
pub fn murrelet_livecode_derive_all(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let ast_receiver = LivecodeReceiver::from_derive_input(&ast).unwrap();

    let livecode = livecode_parse_ast(ast_receiver.clone());
    let nested = nestedit_parse_ast(ast_receiver.clone());
    let lerpable = lerpable_parse_ast(ast_receiver.clone());
    let boop = boop_parse_ast(ast_receiver.clone());
    let lazy = lazy_parse_ast(ast_receiver.clone());

    quote!(
        #livecode
        #nested
        #lerpable
        #boop
        #lazy
    )
    .into()
}

// because I'm using the name "Livecode" to mean alll the things...
// useful to avoid recursion for things like lazy that need to also generate
// just the livecode/unitcell deserializer
#[proc_macro_derive(LivecodeOnly, attributes(livecode))]
pub fn murrelet_livecode_derive_livecode(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let ast_receiver = LivecodeReceiver::from_derive_input(&ast).unwrap();

    // livecode_parse_ast(ast_receiver.clone()).into()

    // and then i realized i still need nested and lerpable too....

    let livecode = livecode_parse_ast(ast_receiver.clone());
    let nested = nestedit_parse_ast(ast_receiver.clone());
    let lerpable = lerpable_parse_ast(ast_receiver.clone());

    quote!(
        #livecode
        #nested
        #lerpable
    )
    .into()

}

#[proc_macro_derive(Lazy, attributes(livecode))]
pub fn murrelet_livecode_derive_lazy(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let ast_receiver = LivecodeReceiver::from_derive_input(&ast).unwrap();
    lazy_parse_ast(ast_receiver.clone()).into()
}

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

#[proc_macro_derive(Lerpable, attributes(livecode))]
pub fn murrelet_livecode_derive_lerpable(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    let ast_receiver = LivecodeReceiver::from_derive_input(&ast).unwrap();
    lerpable_parse_ast(ast_receiver.clone()).into()
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

#[proc_macro_derive(LiveGraphics, attributes(graphics))]
pub fn murrelet_livecode_graphics(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as syn::DeriveInput);
    impl_graphics_trait(ast).into()
}
