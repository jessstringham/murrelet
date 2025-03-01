use proc_macro2::TokenStream as TokenStream2;

use quote::quote;

use crate::derive_livecode::update_to_control_ident;

// ugly quick fix, just generate this code
pub(crate) fn top_level_livecode(ident: syn::Ident) -> TokenStream2 {
    let conf_ident = ident.clone();
    let control_ident = update_to_control_ident(ident.clone());

    quote! {
        type LiveCode = LiveCoder<#conf_ident, #control_ident>;

        impl LiveCoderLoader for #control_ident {
            fn _app_config(&self) -> &murrelet_perform::perform::ControlAppConfig { &self.app }

            fn parse(text: &str) -> murrelet_livecode::types::LivecodeResult<Self> {
                serde_yaml::from_str(&text).map_err(|err| {
                    let line = if let Some(location) = err.location() {
                        format!("{},{}", location.line(), location.column())
                    } else {
                        "".to_owned()
                    };
                    murrelet_livecode::types::LivecodeError::SerdeLoc(line, err.to_string())
                })
            }
        }

        impl murrelet_perform::perform::ConfCommon for #conf_ident {
            fn config_app_loc(&self) -> &murrelet_perform::perform::AppConfig { &self.app }
        }

        impl murrelet_perform::perform::CommonTrait for #conf_ident {}
        impl murrelet_perform::perform::CommonTrait for #control_ident {}
        impl murrelet_perform::perform::LiveCodeCommon<#conf_ident> for #control_ident {}
    }
}

// ugly quick fix, just generate this code
pub(crate) fn top_level_livecode_json(ident: syn::Ident) -> TokenStream2 {
    let conf_ident = ident.clone();
    let control_ident = update_to_control_ident(ident.clone());

    quote! {
        type LiveCode = LiveCoder<#conf_ident, #control_ident>;

        impl LiveCoderLoader for #control_ident {
            fn _app_config(&self) -> &murrelet_perform::perform::ControlAppConfig { &self.app }

            fn parse(text: &str) -> murrelet_livecode::types::LivecodeResult<Self> {
                serde_json::from_str(&text).map_err(|err| {
                    LivecodeError::JsonParse(err.to_string())
                })
            }
        }

        impl murrelet_perform::perform::ConfCommon for #conf_ident {
            fn config_app_loc(&self) -> &murrelet_perform::perform::AppConfig { &self.app }
        }

        impl murrelet_perform::perform::CommonTrait for #conf_ident {}
        impl murrelet_perform::perform::CommonTrait for #control_ident {}
        impl murrelet_perform::perform::LiveCodeCommon<#conf_ident> for #control_ident {}


    }
}

// ugly quick fix, just generate this code
pub(crate) fn impl_all_the_traits(ident: syn::Ident) -> TokenStream2 {
    let conf_ident = ident.clone();
    let control_ident = update_to_control_ident(ident.clone());

    quote! {
        impl murrelet_perform::perform::CommonTrait for #conf_ident {}
        impl murrelet_perform::perform::ConfCommon for #conf_ident {
            fn config_app_loc(&self) -> &murrelet_perform::perform::AppConfig { &self.app }
        }
        impl murrelet_perform::perform::CommonTrait for #control_ident {}
        impl murrelet_perform::perform::LiveCodeCommon<#conf_ident> for #control_ident {}
    }
}
