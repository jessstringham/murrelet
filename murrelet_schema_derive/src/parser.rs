use darling::{ast, FromDeriveInput, FromField, FromVariant};
use proc_macro2::TokenStream as TokenStream2;

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
    fn from_noop_struct(idents: StructIdents) -> Self;
    // fn from_name(idents: StructIdents) -> Self;
    fn from_type_struct(idents: StructIdents) -> Self;
    // fn from_recurse_struct_vec(idents: StructIdents) -> Self;

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
    fn from_override_struct(idents: StructIdents, func: &str) -> Self;
    fn from_override_enum(func: &str) -> Self;

    fn make_enum_final(
        idents: ParsedFieldIdent,
        variants: Vec<Self>,
        is_untagged: bool,
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
                    HowToControlThis::Skip => {
                        #[cfg(feature = "debug_logging")]
                        log::info!("-> from_noop_struct");
                        Self::from_noop_struct(idents)
                    }
                    // HowToControlThis::Name => {
                    //     #[cfg(feature = "debug_logging")]
                    //     log::info!("-> from_name");
                    //     Self::from_name(idents)
                    // }
                    HowToControlThis::SchemaType => {
                        #[cfg(feature = "debug_logging")]
                        log::info!("-> from_type_struct");
                        Self::from_type_struct(idents)
                    }
                    HowToControlThis::Override(func) => Self::from_override_struct(idents, &func),
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

        let variants = e.data.clone().take_enum().unwrap();

        // just go through and find ones that wrap around a type, and make sure those types are
        let variants = variants
            .iter()
            .map(|variant| {
                let ident = EnumIdents {
                    // enum_name: name.clone(),
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

        // "external" => quote! {},
        // "internal" => default,
        // "untagged" => quote! {#[serde(untagged)]},
        // _ => default,
        //    let is_external =  match &e.enum_tag.map(|x| x.as_str()) {
        //         Some("external") => true,
        //         None => false,
        //         _ => unimplemented!("enum type not implemented")
        //     };
        let is_untagged = if let Some(enum_tag) = &e.enum_tag {
            if enum_tag.as_str() == "external" {
                true
            } else {
                false
            }
        } else {
            false
        };

        Self::make_enum_final(idents, variants, is_untagged)
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
                    HowToControlThis::SchemaType => {
                        #[cfg(feature = "debug_logging")]
                        log::info!("-> from_newtype_struct");
                        Self::from_newtype_struct(idents, name.clone())
                    }
                    HowToControlThis::Skip => {
                        #[cfg(feature = "debug_logging")]
                        log::info!("-> from_newtype_recurse_struct_vec");
                        Self::from_noop_struct(idents)
                    }
                    // HowToControlThis::Name => {
                    //     #[cfg(feature = "debug_logging")]
                    //     log::info!("-> from_name");
                    //     Self::from_name(idents)
                    // }
                    HowToControlThis::Override(func) => Self::from_override_enum(&func),
                }
            })
            .collect::<Vec<_>>();

        let idents = ParsedFieldIdent { name: name.clone() };

        Self::make_newtype_struct_final(idents, livecodable_fields)
    }
}

#[derive(Debug, FromField, Clone)]
#[darling(attributes(murrelet_schema))]
pub(crate) struct LivecodeFieldReceiver {
    pub(crate) ident: Option<syn::Ident>,
    pub(crate) ty: syn::Type,
    pub(crate) kind: Option<String>,
    pub(crate) func: Option<String>,
    pub(crate) flatten: Option<bool>,
}
impl LivecodeFieldReceiver {
    fn how_to_control_this(&self) -> HowToControlThis {
        if let Some(kind_val) = &self.kind {
            if kind_val == "skip" {
                HowToControlThis::Skip
            // } else if kind_val == "reference" {
            //     HowToControlThis::Name
            } else {
                panic!("unexpected kind")
            }
        // } else if let Some(_) = &self.reference {
        //     HowToControlThis::Name
        } else if let Some(func) = &self.func {
            HowToControlThis::Override(func.to_owned())
        } else {
            HowToControlThis::SchemaType
        }
    }
}

// for enums
#[derive(Debug, FromVariant, Clone)]
#[darling(attributes(murrelet_schema))]
pub(crate) struct LivecodeVariantReceiver {
    pub(crate) ident: syn::Ident,
    pub(crate) fields: ast::Fields<LivecodeFieldReceiver>,
}

#[derive(Debug, Clone, FromDeriveInput)]
#[darling(attributes(murrelet_schema), supports(any))]
pub(crate) struct LivecodeReceiver {
    ident: syn::Ident,
    data: ast::Data<LivecodeVariantReceiver, LivecodeFieldReceiver>,
    enum_tag: Option<String>,
}
impl LivecodeReceiver {}

// represents an enum
pub(crate) struct EnumIdents {
    // pub(crate) enum_name: syn::Ident,
    pub(crate) data: LivecodeVariantReceiver,
}

impl EnumIdents {}

#[derive(Clone, Debug)]
pub struct StructIdents {
    pub(crate) data: LivecodeFieldReceiver,
}
impl StructIdents {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum HowToControlThis {
    Skip,       // just do the default values
    SchemaType, // build a gui for this type
    // GUIVec, // GUI for a list
    // Name, // a referenced thing,
    Override(String),
}
