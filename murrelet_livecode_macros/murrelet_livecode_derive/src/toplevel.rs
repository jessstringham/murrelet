use proc_macro2::TokenStream as TokenStream2;

use quote::quote;

use crate::{derive_boop::update_to_boop_ident, derive_livecode::update_to_control_ident};

// ugly quick fix, just generate this code
pub(crate) fn top_level_livecode(ident: syn::Ident) -> TokenStream2 {
    let conf_ident = ident.clone();
    let control_ident = update_to_control_ident(ident.clone());
    let boop_ident = update_to_boop_ident(ident);

    quote! {
        type LiveCode = LiveCoder<#conf_ident, #control_ident, #boop_ident>;

        impl LiveCoderLoader for #control_ident {
            fn _app_config(&self) -> &murrelet_perform::perform::ControlAppConfig { &self.app }

            fn parse(text: &str) -> Result<Self, serde_yaml::Error> {
                serde_yaml::from_str(&text)
            }
        }

        impl murrelet_perform::perform::CommonTrait for #conf_ident {}
        impl murrelet_perform::perform::ConfCommon<#boop_ident> for #conf_ident {}
        impl murrelet_perform::perform::CommonTrait for #boop_ident {}
        impl murrelet_perform::perform::BoopConfCommon<#conf_ident> for #boop_ident {}
        impl murrelet_perform::perform::CommonTrait for #control_ident {}
        impl murrelet_perform::perform::LiveCodeCommon<#conf_ident> for #control_ident {}
    }
}

// ugly quick fix, just generate this code
pub(crate) fn impl_all_the_traits(ident: syn::Ident) -> TokenStream2 {
    let conf_ident = ident.clone();
    let control_ident = update_to_control_ident(ident.clone());
    let boop_ident = update_to_boop_ident(ident);

    quote! {
        impl murrelet_perform::perform::CommonTrait for #conf_ident {}
        impl murrelet_perform::perform::ConfCommon<#boop_ident> for #conf_ident {}
        impl murrelet_perform::perform::CommonTrait for #boop_ident {}
        impl murrelet_perform::perform::BoopConfCommon<#conf_ident> for #boop_ident {}
        impl murrelet_perform::perform::CommonTrait for #control_ident {}
        impl murrelet_perform::perform::LiveCodeCommon<#conf_ident> for #control_ident {}
    }
}
