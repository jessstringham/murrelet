use darling::{ast, FromDeriveInput, FromField, FromMeta, FromVariant};
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
    fn from_type_struct(
        idents: StructIdents,
        how_to_control_this_type: &HowToControlThisType,
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
    fn from_override_struct(idents: StructIdents, func: &str, rn_count: usize) -> Self;
    fn from_override_enum(func: &str, rn_count: usize) -> Self;

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
                    HowToControlThis::Override(func, count) => {
                        Self::from_override_struct(idents, &func, count)
                    }
                    HowToControlThis::Normal => panic!("should have an annotation"), //.,
                    HowToControlThis::Type(how_to_control_this_type) => {
                        Self::from_type_struct(idents, &how_to_control_this_type)
                    }
                    HowToControlThis::Default => todo!(),
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
                    HowToControlThis::Normal => {
                        #[cfg(feature = "debug_logging")]
                        log::info!("-> from_newtype_struct");
                        Self::from_newtype_struct(idents, name.clone())
                    }
                    HowToControlThis::Default => {
                        #[cfg(feature = "debug_logging")]
                        log::info!("-> from_noop_struct");
                        Self::from_noop_struct(idents)
                    }
                    HowToControlThis::Type(_) => panic!("hm, hsouldn't have a type here"),
                    HowToControlThis::Override(func, count) => {
                        Self::from_override_enum(&func, count)
                    }
                }
            })
            .collect::<Vec<_>>();

        let idents = ParsedFieldIdent { name: name.clone() };

        Self::make_newtype_struct_final(idents, livecodable_fields)
    }
}

#[derive(Debug, FromMeta, Clone)]
pub struct OverrideFn {
    func: String,
    count: usize,
}

#[derive(Debug, FromField, Clone)]
#[darling(attributes(murrelet_gen))]
pub(crate) struct LivecodeFieldReceiver {
    pub(crate) ident: Option<syn::Ident>,
    pub(crate) ty: syn::Type,
    #[darling(default, rename = "override")]
    pub(crate) override_fn: Option<OverrideFn>,
    #[darling(default)]
    pub(crate) method_bool: Option<RandMethodBool>,
    #[darling(default)]
    pub(crate) method_f32: Option<RandMethodF32>,
    #[darling(default)]
    pub(crate) method_vec2: Option<RandMethodVec2>,
    #[darling(default)]
    pub(crate) method_vec: Option<RandMethodVec>,
    #[darling(default)]
    pub(crate) method_color: Option<RandMethodColor>,
}
impl LivecodeFieldReceiver {
    fn how_to_control_this(&self) -> HowToControlThis {
        let mut method_counts = 0;
        if self.override_fn.is_some() {
            method_counts += 1
        };
        if self.method_bool.is_some() {
            method_counts += 1
        };
        if self.method_f32.is_some() {
            method_counts += 1
        };
        if self.method_vec2.is_some() {
            method_counts += 1
        };
        if self.method_vec.is_some() {
            method_counts += 1
        };
        if self.method_color.is_some() {
            method_counts += 1
        };

        // only one should be
        if method_counts > 1 {
            panic!("more than one method or override specified!");
        }

        if let Some(OverrideFn { func, count }) = &self.override_fn {
            match func.as_str() {
                "default" => HowToControlThis::Default,
                _ => HowToControlThis::Override(func.clone(), *count),
            }
        } else if let Some(r) = self.method_bool {
            HowToControlThis::Type(HowToControlThisType::Bool(r))
        } else if let Some(r) = &self.method_f32 {
            HowToControlThis::Type(HowToControlThisType::F32(r.clone()))
        } else if let Some(r) = &self.method_vec2 {
            HowToControlThis::Type(HowToControlThisType::Vec2(r.clone()))
        } else if let Some(r) = self.method_vec {
            HowToControlThis::Type(HowToControlThisType::Vec(r))
        } else if let Some(r) = self.method_color {
            HowToControlThis::Type(HowToControlThisType::Color(r))
        } else {
            HowToControlThis::Normal
        }
    }
}

#[derive(Debug, Copy, Clone, FromMeta)]
pub enum RandMethodBool {
    Binomial {
        pct: f32, // true
    },
}

#[derive(Debug, Clone, FromMeta)]
pub enum RandMethodF32 {
    Uniform { start: syn::Expr, end: syn::Expr },
}

#[derive(Debug, Clone, FromMeta)]
pub enum RandMethodVec2 {
    UniformGrid {
        x: syn::Expr,
        y: syn::Expr,
        width: f32,
        height: f32,
    },
    Circle {
        x: syn::Expr,
        y: syn::Expr,
        radius: f32,
    },
}

#[derive(Debug, Copy, Clone, FromMeta)]
pub enum RandMethodVec {
    Length { min: usize, max: usize, inside_fn: String },
}

#[derive(Debug, Copy, Clone, FromMeta)]
pub enum RandMethodColor {
    Normal,
    Transparency,
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
    Normal,
    Type(HowToControlThisType),
    Default, // just do the default values
    Override(String, usize),
}

#[derive(Clone, Debug)]
pub(crate) enum HowToControlThisType {
    Bool(RandMethodBool),
    F32(RandMethodF32),
    Vec2(RandMethodVec2),
    Vec(RandMethodVec),
    Color(RandMethodColor),
}
