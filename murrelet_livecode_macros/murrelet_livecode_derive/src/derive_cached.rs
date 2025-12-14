use darling::{ast, FromDeriveInput, FromField, FromVariant};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::DeriveInput;

use crate::parser::ident_from_type;

#[derive(Debug, FromField, Clone)]
#[darling(attributes(cached))]
pub(crate) struct LivecodeFieldReceiver {
    pub(crate) ident: Option<syn::Ident>,
    pub(crate) ty: syn::Type,
}

// for enums
#[derive(Debug, FromVariant, Clone)]
#[darling(attributes(cached))]
pub(crate) struct LivecodeVariantReceiver {}

#[derive(Debug, Clone, FromDeriveInput)]
#[darling(attributes(cached), supports(struct_named))]
pub(crate) struct LivecodeReceiver {
    ident: syn::Ident,
    data: ast::Data<LivecodeVariantReceiver, LivecodeFieldReceiver>,
    // ctrl: Option<String>, // this one should be the struct name...
}

pub fn impl_cache_traits(ast: DeriveInput) -> TokenStream2 {
    let ast_receiver = LivecodeReceiver::from_derive_input(&ast).unwrap();

    match &ast_receiver.data {
        ast::Data::Enum(_) => unreachable!("hm, only works on structs"),
        ast::Data::Struct(fields) => parse_cache(&ast_receiver.ident, &fields.fields),
    }
}

fn parse_cache(name: &syn::Ident, fields: &[LivecodeFieldReceiver]) -> TokenStream2 {


    let mut getter_funcs: Vec<TokenStream2> = vec![];
    let mut check_funcs: Vec<TokenStream2> = vec![];
    let mut init_funcs: Vec<TokenStream2> = vec![];
    let mut to_be_filled_funcs: Vec<TokenStream2> = vec![];
    let mut conf_arguments: Vec<TokenStream2> = vec![];
    for f in fields {
        if let Some(ident) = &f.ident {
            let ident = ident.clone();
            let data = ident_from_type(&f.ty);

            // if it uses our type, we use that is our giveaway
            if data.main_type.to_string().eq("CachedCompute") {
                // there should be a function called compute_$ident

                let expected_compute_name = format!("compute_{}", ident);
                let expected_compute_ident = syn::Ident::new(&expected_compute_name, ident.span());

                let inside_type = data.inside_type().to_quote();


                let func = quote! {
                    fn #ident(&self) -> &#inside_type {
                        self.#ident.get_or_init(|| self.#expected_compute_ident())
                    }
                };

                getter_funcs.push(func);

                let check = quote!{
                    self.#ident.has_been_set()
                };

                check_funcs.push(check);

                let init = quote!{
                    self.#ident()
                };

                init_funcs.push(init);

                let to_be_filled = quote!{
                    #ident: CachedCompute::new()
                };

                to_be_filled_funcs.push(to_be_filled);
            } else {

                // they're passed in with the same name in the arguments
                let orig_type = f.ty.clone();
                let new_conf_argument = quote!{
                    #ident: #orig_type
                };
                conf_arguments.push(new_conf_argument);
                let to_be_filled = quote!{
                    #ident
                };
                to_be_filled_funcs.push(to_be_filled);

            }
        }
    }

    quote! {
        impl #name {
            #(#getter_funcs)*

            fn cached_has_been_set(&self) -> bool {
                true #( && #check_funcs )*
            }

            fn init_all_cached(&self) {
                #(#init_funcs;)*
            }

            fn new(#(#conf_arguments,)*) -> Self {
                #name {
                    #(#to_be_filled_funcs,)*
                }
            }

        }
    }
}
