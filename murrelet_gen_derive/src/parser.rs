use darling::{FromDeriveInput, FromField, FromVariant, ast};
use proc_macro2::TokenStream as TokenStream2;

use crate::GenMethod;

#[derive(Debug)]
pub(crate) struct ParsedFieldIdent {
    pub(crate) name: syn::Ident,
}

// trait and helpers needed to parse a variety of objects
pub(crate) trait GenFinal
where
    Self: Sized,
{
    fn from_newtype_struct(_idents: StructIdents, parent_ident: syn::Ident) -> Self;
    fn from_unnamed_enum(idents: EnumIdents) -> Self;
    fn from_unit_enum(idents: EnumIdents) -> Self;
    fn from_type_struct(idents: StructIdents, how_to_control_this_type: &GenMethod) -> Self;
    fn from_type_recurse(
        idents: StructIdents,
        how_to_control_outer_type: &GenMethod,
        how_to_control_inner_type: &GenMethod,
    ) -> Self;

    fn from_ast(ast_receiver: LivecodeReceiver) -> TokenStream2 {
        match ast_receiver.data {
            ast::Data::Enum(_) => Self::make_enum(&ast_receiver),
            ast::Data::Struct(ast::Fields {
                style: ast::Style::Tuple,
                ..
            }) => Self::make_newtype(&ast_receiver),
            ast::Data::Struct(_) => Self::make_struct(&ast_receiver),
        }
    }
    // fn from_override_struct(
    //     idents: StructIdents,
    //     func: &str,
    //     rn_names: Vec<String>,
    //     rn_count: usize,
    // ) -> Self;

    fn make_enum_final(
        idents: ParsedFieldIdent,
        variants: Vec<Self>,
        variants_receiver: &[LivecodeVariantReceiver],
    ) -> TokenStream2;
    fn make_struct_final(idents: ParsedFieldIdent, variants: Vec<Self>) -> TokenStream2;
    fn make_newtype_struct_final(idents: ParsedFieldIdent, variants: Vec<Self>) -> TokenStream2;

    fn make_struct(s: &LivecodeReceiver) -> TokenStream2 {
        let name = s.ident.clone();

        #[cfg(feature = "debug_logging")]
        log::info!("{}::make_struct {}", Self::classname(), name.to_string());

        // shouldn't be calling this with something that's not a struct..
        let fields = s.data.clone().take_struct().unwrap();

        let livecodable_fields = fields
            .iter()
            .map(|field| {
                let idents = StructIdents {
                    data: field.clone(),
                };

                match field.how_to_control_this() {
                    // HowToControlThis::Override(func, names, count) => {
                    //     Self::from_override_struct(idents, &func, names, count)
                    // }
                    HowToControlThis::Type(how_to_control_this_type) => {
                        Self::from_type_struct(idents, &how_to_control_this_type)
                    }
                    HowToControlThis::Recurse(outer, inner) => {
                        Self::from_type_recurse(idents, &outer, &inner)
                    }
                }
            })
            .collect::<Vec<_>>();

        let idents = ParsedFieldIdent { name: name.clone() };

        Self::make_struct_final(idents, livecodable_fields)
    }

    fn make_enum(e: &LivecodeReceiver) -> TokenStream2 {
        let name = e.ident.clone();

        #[cfg(feature = "debug_logging")]
        log::info!("{}::make_enum {}", Self::classname(), name.to_string());

        let variants_receiver = e.data.clone().take_enum().unwrap();

        // just go through and find ones that wrap around a type, and make sure those types are
        let variants = variants_receiver
            .iter()
            .map(|variant| {
                let ident = EnumIdents {
                    enum_name: name.clone(),
                    data: variant.clone(),
                };

                match variant.fields.style {
                    ast::Style::Tuple => Self::from_unnamed_enum(ident),
                    ast::Style::Struct => panic!("enum named fields not supported yet"),
                    ast::Style::Unit => Self::from_unit_enum(ident),
                }
            })
            .collect::<Vec<_>>();

        let idents = ParsedFieldIdent { name: name.clone() };

        Self::make_enum_final(idents, variants, &variants_receiver)
    }

    fn make_newtype(s: &LivecodeReceiver) -> TokenStream2 {
        let name = s.ident.clone();

        #[cfg(feature = "debug_logging")]
        log::info!("{}::make_newtype {}", Self::classname(), name.to_string());

        // shouldn't be calling this with something that's not a struct..
        let fields = s.data.clone().take_struct().unwrap();

        let livecodable_fields = fields
            .iter()
            .map(|field| {
                let idents = StructIdents {
                    data: field.clone(),
                };

                match field.how_to_control_this() {
                    HowToControlThis::Type(_how_to_control_this_type) => {
                        // Self::from_type_struct(idents, &how_to_control_this_type)
                        Self::from_newtype_struct(idents, name.clone())
                    }
                    HowToControlThis::Recurse(_outer, _inner) => {
                        // Self::from_type_recurse(idents, &outer, &inner)
                        Self::from_newtype_struct(idents, name.clone())
                    } // HowToControlThis::Override(func, labels, count) => {
                      //     Self::from_override_struct(idents, &func, labels, count)
                      // }
                }
            })
            .collect::<Vec<_>>();

        let idents = ParsedFieldIdent { name: name.clone() };

        Self::make_newtype_struct_final(idents, livecodable_fields)
    }
}

#[derive(Debug, FromField, Clone)]
#[darling(attributes(murrelet_gen))]
pub(crate) struct LivecodeFieldReceiver {
    pub(crate) ident: Option<syn::Ident>,
    pub(crate) ty: syn::Type,
    pub(crate) method: GenMethod,
    #[darling(default)]
    pub(crate) method_inner: Option<GenMethod>,
}
impl LivecodeFieldReceiver {
    fn how_to_control_this(&self) -> HowToControlThis {
        if let Some(r) = &self.method_inner {
            HowToControlThis::Recurse(self.method.clone(), r.clone())
        } else if matches!(self.method, GenMethod::VecLength { .. }) {
            panic!("vec missing inner")
        } else {
            HowToControlThis::Type(self.method.clone())
        }
    }
}

// for enums
#[derive(Debug, FromVariant, Clone)]
#[darling(attributes(murrelet_gen))]
pub(crate) struct LivecodeVariantReceiver {
    pub(crate) ident: syn::Ident,
    pub(crate) fields: ast::Fields<LivecodeFieldReceiver>,
    pub(crate) weight: f32, // either each field needs something
}

#[derive(Debug, Clone, FromDeriveInput)]
#[darling(attributes(murrelet_gen), supports(any))]
pub(crate) struct LivecodeReceiver {
    ident: syn::Ident,
    data: ast::Data<LivecodeVariantReceiver, LivecodeFieldReceiver>,
}
impl LivecodeReceiver {}

// represents an enum
pub(crate) struct EnumIdents {
    pub(crate) enum_name: syn::Ident,
    pub(crate) data: LivecodeVariantReceiver,
}

#[derive(Clone, Debug)]
pub struct StructIdents {
    pub(crate) data: LivecodeFieldReceiver,
}

#[derive(Clone, Debug)]
pub(crate) enum HowToControlThis {
    Type(GenMethod),
    Recurse(GenMethod, GenMethod),
}
