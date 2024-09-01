use darling::{ast, FromDeriveInput, FromField, FromVariant};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

pub(crate) fn prefix_ident(prefix: &str, name: syn::Ident) -> syn::Ident {
    let lc_name = format!("{}{}", prefix, name);
    syn::Ident::new(&lc_name, name.span())
}

#[derive(Debug)]
pub(crate) struct ParsedFieldIdent {
    pub(crate) new_ident: syn::Ident,
    pub(crate) vis: syn::Visibility,
    pub(crate) name: syn::Ident,
    // things for the entire object, like untagged
    pub(crate) tags: TokenStream2,
}

// trait and helpers needed to parse a variety of objects
pub(crate) trait GenFinal
where
    Self: Sized,
{
    fn from_newtype_struct(_idents: StructIdents, parent_ident: syn::Ident) -> Self;
    fn from_newtype_recurse_struct_vec(_idents: StructIdents) -> Self;
    fn from_unnamed_enum(idents: EnumIdents) -> Self;
    fn from_unit_enum(idents: EnumIdents) -> Self;
    fn from_noop_struct(idents: StructIdents) -> Self;
    fn from_type_struct(idents: StructIdents) -> Self;
    fn from_recurse_struct_vec(idents: StructIdents) -> Self;
    fn from_recurse_struct_struct(idents: StructIdents) -> Self;
    fn from_recurse_struct_unitcell(idents: StructIdents) -> Self;

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

    fn new_ident(name: syn::Ident) -> syn::Ident;
    fn make_enum_final(idents: ParsedFieldIdent, variants: Vec<Self>) -> TokenStream2;
    fn make_struct_final(idents: ParsedFieldIdent, variants: Vec<Self>) -> TokenStream2;
    fn make_newtype_struct_final(idents: ParsedFieldIdent, variants: Vec<Self>) -> TokenStream2;

    fn make_struct(s: &LivecodeReceiver) -> TokenStream2 {
        let name = s.ident.clone();
        let lc_ident = Self::new_ident(name.clone());

        // shouldn't be calling this with something that's not a struct..
        let fields = s.data.clone().take_struct().unwrap();

        let livecodable_fields = fields
            .iter()
            .map(|field| {
                let idents = StructIdents {
                    data: field.clone(),
                };

                match field.how_to_control_this() {
                    // leave this field alone (useful for String and HashMaps)
                    HowToControlThis::WithNone(_) => Self::from_noop_struct(idents),
                    // creating with a set type
                    HowToControlThis::WithType(_, _) => Self::from_type_struct(idents),
                    // creating a Vec<Something>
                    HowToControlThis::WithRecurse(_, RecursiveControlType::Vec) => {
                        Self::from_recurse_struct_vec(idents)
                    }
                    // creating a : Something in livecode
                    HowToControlThis::WithRecurse(_, RecursiveControlType::Struct) => {
                        Self::from_recurse_struct_struct(idents)
                    }
                    // dealing with UnitCell<something>
                    HowToControlThis::WithRecurse(_, RecursiveControlType::UnitCell) => {
                        Self::from_recurse_struct_unitcell(idents)
                    }
                }
            })
            .collect::<Vec<_>>();

        let idents = ParsedFieldIdent {
            new_ident: lc_ident.clone(),
            vis: s.vis.clone(),
            name: name.clone(),
            tags: quote!(), // for now, nothing here
        };

        Self::make_struct_final(idents, livecodable_fields)
    }

    fn make_enum(e: &LivecodeReceiver) -> TokenStream2 {
        let name = e.ident.clone();

        let new_enum_ident = Self::new_ident(e.ident.clone());

        let variants = e.data.clone().take_enum().unwrap();

        // just go through and find ones that wrap around a type, and make sure those types are
        let variants = variants
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

        let idents = ParsedFieldIdent {
            new_ident: new_enum_ident.clone(),
            vis: e.vis.clone(),
            name: name.clone(),
            tags: e.serde_enum_type(),
        };

        Self::make_enum_final(idents, variants)
    }

    fn make_newtype(s: &LivecodeReceiver) -> TokenStream2 {
        let name = s.ident.clone();
        let lc_ident = Self::new_ident(name.clone());

        // shouldn't be calling this with something that's not a struct..
        let fields = s.data.clone().take_struct().unwrap();

        let livecodable_fields = fields
            .iter()
            .map(|field| {
                let idents = StructIdents {
                    data: field.clone(),
                };

                match field.how_to_control_this() {
                    // don't change anything
                    // HowToControlThis::WithNone(_) => Self::from_noop_struct(idents),
                    // creating with a set type
                    HowToControlThis::WithType(_, _) => {
                        Self::from_newtype_struct(idents, name.clone())
                    }
                    // creating a Vec<Something>
                    HowToControlThis::WithRecurse(_, RecursiveControlType::Vec) => {
                        Self::from_newtype_recurse_struct_vec(idents)
                    }
                    // creating a : Something in livecode
                    // HowToControlThis::WithRecurse(_, RecursiveControlType::Struct) => Self::from_recurse_struct_struct(idents),
                    // dealing with UnitCell<something>
                    // HowToControlThis::WithRecurse(_, RecursiveControlType::UnitCell) => Self::from_recurse_struct_unitcell(idents),
                    _ => panic!("newtype for this kind isn't implemented yet"),
                }
            })
            .collect::<Vec<_>>();

        let idents = ParsedFieldIdent {
            new_ident: lc_ident.clone(),
            vis: s.vis.clone(),
            name: name.clone(),
            tags: quote!(), // for now, nothing here
        };

        Self::make_newtype_struct_final(idents, livecodable_fields)
    }
}

#[derive(Debug, FromField, Clone)]
#[darling(attributes(livecode))]
pub(crate) struct LivecodeFieldReceiver {
    pub(crate) ident: Option<syn::Ident>,
    pub(crate) ty: syn::Type,
    pub(crate) serde_default: Option<String>, // parsed and passed on to serde
    pub(crate) serde_opts: Option<String>,
    pub(crate) kind: Option<String>, // used of override type
    pub(crate) ctx: Option<String>,
    pub(crate) src: Option<String>,    // sequencer
    pub(crate) prefix: Option<String>, // what to prefix the src with
    pub(crate) f32min: Option<f32>,    // only used if it's a f32
    pub(crate) f32max: Option<f32>,
}
impl LivecodeFieldReceiver {
    fn how_to_control_this(&self) -> HowToControlThis {
        // first check if 'kind' is set
        if let Some(kind) = &self.kind {
            HowToControlThis::from_kind(kind)
        } else {
            let type_idents = ident_from_type(&self.ty);
            HowToControlThis::from_type_str(type_idents.main_type.to_string().as_ref())
        }
    }

    fn parse_serde(&self) -> Option<SerdeDefault> {
        self.serde_default
            .as_ref()
            .map(|serde_d| match serde_d.as_ref() {
                "default" => SerdeDefault::DefaultImpl,
                "zeros" => SerdeDefault::Zeros,
                "0" => SerdeDefault::Zeros,
                "false" => SerdeDefault::Zeros,
                "ones" => SerdeDefault::Ones,
                "1" => SerdeDefault::Ones,
                "true" => SerdeDefault::Ones,
                "empty" => SerdeDefault::Empty,
                y => SerdeDefault::CustomFunction(y.to_string()),
            })
    }

    fn serde_tokens(&self, is_unit_cell: bool) -> TokenStream2 {
        let how = self.how_to_control_this();
        let maybe_serde = self.parse_serde();
        let default = if let Some(serde) = maybe_serde {
            match serde {
                SerdeDefault::CustomFunction(c) => quote! {#[serde(default=#c)]},
                SerdeDefault::DefaultImpl => quote! {#[serde(default)]},
                SerdeDefault::Empty => {
                    quote! {#[serde(default="murrelet_livecode::livecode::empty_vec")]}
                }
                _ => {
                    let serde_func = serde.from_control_type(is_unit_cell, how.get_control_type());
                    quote! {#[serde(default=#serde_func)]}
                }
            }
        } else {
            quote! {}
        };

        // now match other fields that are just passed directly through
        let other_opts = if let Some(opts) = &self.serde_opts {
            quote! { #[serde(#opts)] }
        } else {
            quote! {}
        };

        quote! {
            #default
            #other_opts
        }
    }
}

#[derive(Debug, FromVariant, Clone)]
#[darling(attributes(livecode))]
pub(crate) struct LivecodeVariantReceiver {
    pub(crate) ident: syn::Ident,
    pub(crate) fields: ast::Fields<LivecodeFieldReceiver>,
}

#[derive(Debug, Clone, FromDeriveInput)]
#[darling(attributes(livecode), supports(any))]
pub(crate) struct LivecodeReceiver {
    ident: syn::Ident,
    vis: syn::Visibility,
    data: ast::Data<LivecodeVariantReceiver, LivecodeFieldReceiver>,
    enum_tag: Option<String>,
}
impl LivecodeReceiver {
    fn serde_enum_type(&self) -> TokenStream2 {
        let default = quote! {#[serde(tag = "type")]};
        if let Some(ex) = &self.enum_tag {
            match ex.as_str() {
                "external" => quote! {},
                "internal" => default,
                "untagged" => quote! {#[serde(untagged)]},
                _ => default,
            }
        } else {
            default
        }
    }
}

pub(crate) struct EnumIdents {
    pub(crate) enum_name: syn::Ident,
    pub(crate) data: LivecodeVariantReceiver,
    // pub(crate) name: syn::Ident,
    // pub(crate) variant_ident: syn::Ident
}

impl EnumIdents {
    pub(crate) fn variant_ident(&self) -> syn::Ident {
        self.data.ident.clone()
    }

    pub(crate) fn enum_ident(&self) -> syn::Ident {
        self.enum_name.clone()
    }
}

#[derive(Clone, Debug)]
pub struct StructIdents {
    pub(crate) data: LivecodeFieldReceiver,
}
impl StructIdents {
    pub(crate) fn name(&self) -> syn::Ident {
        self.data.ident.clone().unwrap()
    }

    pub(crate) fn orig_ty(&self) -> syn::Type {
        self.data.ty.clone()
    }

    // todo, is_unitcell is ugly..
    pub(crate) fn serde(&self, is_unitcell: bool) -> TokenStream2 {
        self.data.serde_tokens(is_unitcell)
    }

    pub(crate) fn how_to_control_this(&self) -> HowToControlThis {
        self.data.how_to_control_this()
    }

    pub(crate) fn control_type(&self) -> ControlType {
        self.how_to_control_this().get_control_type()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlType {
    F32,
    Bool,
    F32_2,
    F32_3,
    Color,
    ColorUnclamped,
    LazyNodeF32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RecursiveControlType {
    Struct,
    Vec,
    UnitCell, // special type that builds up an expression context
              // Array,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OverrideOrInferred {
    Override,
    Inferred,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HowToControlThis {
    WithType(OverrideOrInferred, ControlType),
    WithNone(OverrideOrInferred),
    WithRecurse(OverrideOrInferred, RecursiveControlType),
}
impl HowToControlThis {
    pub(crate) fn get_control_type(&self) -> ControlType {
        match self {
            HowToControlThis::WithType(_, x) => *x,
            HowToControlThis::WithNone(_) => panic!("control none"),
            HowToControlThis::WithRecurse(_, _) => panic!("control recurse"),
        }
    }

    pub(crate) fn from_kind(value: &str) -> HowToControlThis {
        match value {
            "none" => HowToControlThis::WithNone(OverrideOrInferred::Override),
            "bool" => HowToControlThis::WithType(OverrideOrInferred::Override, ControlType::F32),
            "f32" => HowToControlThis::WithType(OverrideOrInferred::Override, ControlType::F32),
            "f32;2" => HowToControlThis::WithType(OverrideOrInferred::Override, ControlType::F32_2),
            "[f32;2]" => {
                HowToControlThis::WithType(OverrideOrInferred::Override, ControlType::F32_2)
            }
            "color" => HowToControlThis::WithType(OverrideOrInferred::Override, ControlType::Color),
            "color unclamped" => HowToControlThis::WithType(
                OverrideOrInferred::Override,
                ControlType::ColorUnclamped,
            ),
            "s" => HowToControlThis::WithRecurse(
                OverrideOrInferred::Override,
                RecursiveControlType::Struct,
            ),
            "v" => HowToControlThis::WithRecurse(
                OverrideOrInferred::Override,
                RecursiveControlType::Vec,
            ),
            // "expr" => {
            //     HowToControlThis::WithType(OverrideOrInferred::Override, ControlType::EvalExpr)
            // }
            "unitcell" => HowToControlThis::WithRecurse(
                OverrideOrInferred::Override,
                RecursiveControlType::UnitCell,
            ),
            _ => panic!("parsing kind, {:?} not none, bool, f32, f32;2, s", value),
        }
    }

    pub(crate) fn from_type_str(value: &str) -> HowToControlThis {
        match value {
            "f32" => HowToControlThis::WithType(OverrideOrInferred::Inferred, ControlType::F32),
            "f64" => HowToControlThis::WithType(OverrideOrInferred::Inferred, ControlType::F32),
            "usize" => HowToControlThis::WithType(OverrideOrInferred::Inferred, ControlType::F32),
            "u32" => HowToControlThis::WithType(OverrideOrInferred::Inferred, ControlType::F32),
            "u64" => HowToControlThis::WithType(OverrideOrInferred::Inferred, ControlType::F32),
            "u8" => HowToControlThis::WithType(OverrideOrInferred::Inferred, ControlType::F32),
            "i32" => HowToControlThis::WithType(OverrideOrInferred::Inferred, ControlType::F32),
            "bool" => HowToControlThis::WithType(OverrideOrInferred::Inferred, ControlType::Bool),
            "Vec2" => HowToControlThis::WithType(OverrideOrInferred::Inferred, ControlType::F32_2),
            "Vec" => HowToControlThis::WithRecurse(
                OverrideOrInferred::Inferred,
                RecursiveControlType::Vec,
            ),
            "Vec3" => HowToControlThis::WithType(OverrideOrInferred::Inferred, ControlType::F32_3),
            "String" => HowToControlThis::WithNone(OverrideOrInferred::Inferred),
            // some special types from this library
            "MurreletColor" => {
                HowToControlThis::WithType(OverrideOrInferred::Inferred, ControlType::Color)
            }
            "AdditionalContextNode" => HowToControlThis::WithNone(OverrideOrInferred::Inferred),
            "UnitCells" => HowToControlThis::WithRecurse(
                OverrideOrInferred::Inferred,
                RecursiveControlType::UnitCell,
            ),
            "LazyNodeF32" => {
                HowToControlThis::WithType(OverrideOrInferred::Inferred, ControlType::LazyNodeF32)
            }
            // _ => HowToControlThis::WithNone(OverrideOrInferred::Inferred)
            _ => HowToControlThis::WithRecurse(
                OverrideOrInferred::Inferred,
                RecursiveControlType::Struct,
            ),
        }
    }
}

// serde things

#[derive(Clone, Debug, PartialEq, Eq)]
enum SerdeDefault {
    Zeros,
    Ones,
    CustomFunction(String),
    Empty,       // empty vec
    DefaultImpl, // use Default
}
impl SerdeDefault {
    fn from_control_type(&self, is_unit_cell: bool, ty: ControlType) -> String {
        if is_unit_cell {
            match (ty, self) {
                (ControlType::Bool, SerdeDefault::Zeros) => {
                    "murrelet_livecode::unitcells::_auto_default_bool_false_unitcell".to_string()
                }
                (ControlType::Bool, SerdeDefault::Ones) => {
                    "murrelet_livecode::unitcells::_auto_default_bool_true_unitcell".to_string()
                }
                (ControlType::F32_2, SerdeDefault::Zeros) => {
                    "murrelet_livecode::unitcells::_auto_default_vec2_0_unitcell".to_string()
                }
                (ControlType::F32_2, SerdeDefault::Ones) => {
                    "murrelet_livecode::unitcells::_auto_default_vec2_1_unitcell".to_string()
                }
                (ControlType::F32, SerdeDefault::Zeros) => {
                    "murrelet_livecode::unitcells::_auto_default_0_unitcell".to_string()
                }
                (ControlType::F32, SerdeDefault::Ones) => {
                    "murrelet_livecode::unitcells::_auto_default_1_unitcell".to_string()
                }
                (ControlType::F32_3, SerdeDefault::Zeros) => {
                    "murrelet_livecode::unitcells::_auto_default_vec3_0_unitcell".to_string()
                }
                (ControlType::Color, SerdeDefault::Zeros) => {
                    "murrelet_livecode::unitcells::_auto_default_color_4_unitcell".to_string()
                }
                _ => {
                    todo!(
                        "just need to implement serde default for unit cell {:?}, {:?}",
                        ty,
                        self
                    )
                }
            }
        } else {
            // todo, the custom func is being handled in two places...
            match (ty, self) {
                (ControlType::F32, SerdeDefault::Zeros) => {
                    "murrelet_livecode::livecode::_auto_default_f32_0".to_string()
                }
                (ControlType::F32, SerdeDefault::Ones) => {
                    "murrelet_livecode::livecode::_auto_default_f32_0".to_string()
                }
                (ControlType::F32, SerdeDefault::CustomFunction(x)) => x.clone(),
                (ControlType::Bool, SerdeDefault::Zeros) => {
                    "murrelet_livecode::livecode::_auto_default_bool_false".to_string()
                }
                (ControlType::Bool, SerdeDefault::Ones) => {
                    "murrelet_livecode::livecode::_auto_default_bool_true".to_string()
                }
                (ControlType::Bool, SerdeDefault::CustomFunction(x)) => x.clone(),
                (ControlType::F32_2, SerdeDefault::Zeros) => {
                    "murrelet_livecode::livecode::_auto_default_vec2_0".to_string()
                }
                (ControlType::F32_2, SerdeDefault::Ones) => {
                    "murrelet_livecode::livecode::_auto_default_vec2_1".to_string()
                }
                (ControlType::F32_2, SerdeDefault::CustomFunction(x)) => x.clone(),
                (ControlType::F32_3, SerdeDefault::CustomFunction(x)) => x.clone(),
                (ControlType::Color, SerdeDefault::CustomFunction(x)) => x.clone(),
                (ControlType::ColorUnclamped, SerdeDefault::CustomFunction(x)) => x.to_string(),
                _ => panic!("not implemented yet, need {:?} {:?}", ty, self),
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct DataFromType {
    pub(crate) main_type: syn::Ident,
    pub(crate) second_type: Option<syn::Ident>,
}
impl DataFromType {
    pub(crate) fn new(main_type: syn::Ident, second_type: Option<syn::Ident>) -> Self {
        Self {
            main_type,
            second_type,
        }
    }

    pub(crate) fn has_second(&self) -> bool {
        self.second_type.is_none()
    }
}

pub(crate) fn ident_from_type(t: &syn::Type) -> DataFromType {
    match t {
        syn::Type::Path(syn::TypePath { path, .. }) => {
            let s = path.segments.last().unwrap();
            let main_type = s.ident.clone();

            let second_type =
                if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                    args,
                    ..
                }) = s.arguments.clone()
                {
                    if let syn::GenericArgument::Type(other_ty) = args.first().unwrap() {
                        let data_from_type = ident_from_type(other_ty);
                        assert!(data_from_type.has_second(), "nested types not implemented");
                        Some(data_from_type.main_type)
                    } else {
                        panic!("not implemented yet {:?}", args);
                    }
                } else {
                    None
                };

            // if arguments

            DataFromType::new(main_type, second_type)
        }
        x => panic!("no name for type {:?}", x),
    }
}
