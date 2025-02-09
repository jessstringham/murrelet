use darling::{ast, FromDeriveInput, FromField, FromVariant};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

const DEBUG_THIS: bool = false;

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
    pub(crate) lazy_enum_tag: TokenStream2, // argh, special case for enums right now
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
    fn from_recurse_struct_lazy(idents: StructIdents) -> Self;

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

        if DEBUG_THIS {
            println!("{}::make_struct {}", Self::classname(), name);
        }

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
                    HowToControlThis::WithNone(_) => {
                        if DEBUG_THIS {
                            println!("-> from_noop_struct");
                        }
                        Self::from_noop_struct(idents)
                    }
                    // creating with a set type
                    HowToControlThis::WithType(_, _) => {
                        if DEBUG_THIS {
                            println!("-> from_type_struct");
                        }
                        Self::from_type_struct(idents)
                    }
                    // creating a Vec<Something>
                    HowToControlThis::WithRecurse(_, RecursiveControlType::Vec) => {
                        if DEBUG_THIS {
                            println!("-> from_recurse_struct_vec");
                        }
                        Self::from_recurse_struct_vec(idents)
                    }
                    // creating a : Something in livecode
                    HowToControlThis::WithRecurse(_, RecursiveControlType::Struct) => {
                        if DEBUG_THIS {
                            println!("-> from_recurse_struct_struct");
                        }
                        Self::from_recurse_struct_struct(idents)
                    }
                    HowToControlThis::WithRecurse(_, RecursiveControlType::StructLazy) => {
                        if DEBUG_THIS {
                            println!("-> from_recurse_struct_lazy");
                        }
                        Self::from_recurse_struct_lazy(idents)
                    }

                    // dealing with UnitCell<something>
                    HowToControlThis::WithRecurse(_, RecursiveControlType::UnitCell) => {
                        if DEBUG_THIS {
                            println!("-> from_recurse_struct_unitcell");
                        }
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
            lazy_enum_tag: quote!(),
        };

        Self::make_struct_final(idents, livecodable_fields)
    }

    fn make_enum(e: &LivecodeReceiver) -> TokenStream2 {
        let name = e.ident.clone();

        if DEBUG_THIS {
            println!("{}::make_enum {}", Self::classname(), name);
        }

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
            lazy_enum_tag: e.enum_back_to_quote_for_lazy(), // this is important for Lazy
        };

        Self::make_enum_final(idents, variants)
    }

    fn classname() -> String {
        std::any::type_name::<Self>()
            .split("::")
            .last()
            .unwrap_or("")
            .to_owned()
    }

    fn make_newtype(s: &LivecodeReceiver) -> TokenStream2 {
        let name = s.ident.clone();

        if DEBUG_THIS {
            println!("{}::make_newtype {}", Self::classname(), name);
        }

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
                        if DEBUG_THIS {
                            println!("-> from_newtype_struct");
                        }
                        Self::from_newtype_struct(idents, name.clone())
                    }
                    // creating a Vec<Something>
                    HowToControlThis::WithRecurse(_, RecursiveControlType::Vec) => {
                        if DEBUG_THIS {
                            println!("-> from_newtype_recurse_struct_vec");
                        }
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
            tags: quote!(),
            lazy_enum_tag: quote!(), // for now, nothing here
        };

        Self::make_newtype_struct_final(idents, livecodable_fields)
    }
}

fn lazy_version_of_default_serde(c: &str) -> String {
    if c.ends_with("_lazy") {
        c.to_owned() // already good!
    } else {
        format!("{}_lazy", c)
    }
}

#[derive(Debug, FromField, Clone)]
#[darling(attributes(livecode))]
pub(crate) struct LivecodeFieldReceiver {
    pub(crate) ident: Option<syn::Ident>,
    pub(crate) ty: syn::Type,
    pub(crate) serde_default: Option<String>, // parsed and passed on to serde
    pub(crate) lazy_serde_default: Option<String>, // used internally, for lazy things to get converted to livecode lazy correctly
    pub(crate) serde_opts: Option<String>,
    pub(crate) kind: Option<String>, // used of override type
    pub(crate) ctx: Option<String>,
    pub(crate) src: Option<String>,    // sequencer
    pub(crate) prefix: Option<String>, // what to prefix the src with
    pub(crate) f32min: Option<f32>,    // only used if it's a f32
    pub(crate) f32max: Option<f32>,
}
impl LivecodeFieldReceiver {
    fn back_to_quote_for_lazy(&self) -> TokenStream2 {
        let r = vec![
            self.ctx.as_ref().map(|x| {
                let ctx = x.to_string();
                quote! {ctx = #ctx}
            }),
            self.src.as_ref().map(|x| {
                let src = x.to_string();
                quote! {src = #src}
            }),
            self.kind.as_ref().map(|x| {
                let kind = x.to_string();
                quote! {kind = #kind}
            }),
            // serde default is where it needs to change!
            self.serde_default.as_ref().map(|x| {
                let serde_default = x.to_string();
                quote! {lazy_serde_default = #serde_default}
            }),
            self.serde_opts.as_ref().map(|x| {
                let serde_opts = x.to_string();
                quote! {serde_opts = #serde_opts}
            }),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        if !r.is_empty() {
            quote! { #[livecode(#(#r,)*)] }
        } else {
            quote! {}
        }

        // let b = if let Some(serde_default) = &self.serde_default {
        //     quote! {#[serde(default = #serde_default)] #a}
        // } else {
        //     a
        // };

        // if let Some(opts) = &self.serde_opts {
        //     quote! {#[serde(#opts)] #b}
        // } else {
        //     b
        // }
    }

    fn how_to_control_this(&self) -> HowToControlThis {
        // first check if 'kind' is set
        if let Some(kind) = &self.kind {
            HowToControlThis::from_kind(kind)
        } else {
            let type_idents = ident_from_type(&self.ty);
            HowToControlThis::from_type_str(type_idents.main_type.to_string().as_ref())
        }
    }

    fn parse_serde(&self, serde_default: Option<&String>) -> Option<SerdeDefault> {
        serde_default.map(|serde_d| match serde_d.as_ref() {
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

    fn serde_tokens(&self) -> TokenStream2 {
        let is_lazy = self.lazy_serde_default.is_some();

        let maybe_serde = if is_lazy {
            self.parse_serde(self.lazy_serde_default.as_ref())
        } else {
            self.parse_serde(self.serde_default.as_ref())
        };

        let default = if let Some(serde) = maybe_serde {
            match serde {
                SerdeDefault::CustomFunction(c) => {
                    let r = if is_lazy {
                        lazy_version_of_default_serde(&c)
                    } else {
                        c
                    };
                    quote! {#[serde(default=#r)]}
                }
                SerdeDefault::DefaultImpl => quote! {#[serde(default)]},
                SerdeDefault::Empty => {
                    // nace and general
                    quote! {#[serde(default="murrelet_livecode::livecode::empty_vec")]}
                }
                _ => {
                    // first check if it's a special thing
                    let how = self.how_to_control_this();

                    if is_lazy {
                        let serde_func = match &how {
                            HowToControlThis::WithRecurse(_, RecursiveControlType::Vec) => {
                                // weird and hardcoded for things like Lazy Vec2, which get turned into Vec<f32>...
                                serde.from_control_type(ControlType::LazyNodeF32, true)
                            }
                            _ => serde.from_control_type(how.get_control_type(), false),
                        };
                        let r = lazy_version_of_default_serde(&serde_func);

                        quote! {#[serde(default=#r)]}
                    } else {
                        let r = serde.from_control_type(how.get_control_type(), false);
                        quote! {#[serde(default=#r)]}
                    }
                }
            }
        } else {
            quote! {}
        };

        // now match other fields that are just passed directly through
        let other_opts = if let Some(opts) = &self.serde_opts {
            match opts.as_str() {
                "flatten" => quote! { #[serde(flatten)] },
                _ => unimplemented!("opt {}", opts),
            }
        } else {
            quote! {}
        };

        quote! {
            #default
            #other_opts
        }
    }
}

// for enums
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

    fn enum_back_to_quote_for_lazy(&self) -> TokenStream2 {
        let r = vec![self.enum_tag.as_ref().map(|x| {
            let enum_tag = x.to_string();
            quote! {enum_tag = #enum_tag}
        })]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

        if !r.is_empty() {
            quote! { #[livecode(#(#r,)*)] }
        } else {
            quote! {}
        }
    }
}

// represents an enum
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

    pub(crate) fn serde(&self) -> TokenStream2 {
        self.data.serde_tokens()
    }

    pub(crate) fn how_to_control_this(&self) -> HowToControlThis {
        self.data.how_to_control_this()
    }

    pub(crate) fn how_to_control_this_is_none(&self) -> bool {
        match self.how_to_control_this() {
            HowToControlThis::WithNone(_) => true,
            _ => false,
        }
    }

    pub(crate) fn control_type(&self) -> ControlType {
        self.how_to_control_this().get_control_type()
    }

    pub(crate) fn back_to_quote(&self) -> TokenStream2 {
        self.data.back_to_quote_for_lazy()
    }

    pub(crate) fn is_serde_flatten(&self) -> bool {
        if let Some(x) = &self.data.serde_opts {
            x == "flatten"
        } else {
            false
        }
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
    StructLazy, // just a way to stop some features from propogating..
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

    pub(crate) fn needs_to_be_evaluated(&self) -> bool {
        match self {
            HowToControlThis::WithType(_, _) => true,
            HowToControlThis::WithRecurse(_, _) => true,
            HowToControlThis::WithNone(_) => false,
        }
    }

    pub(crate) fn is_lazy(&self) -> bool {
        match self {
            HowToControlThis::WithRecurse(_, RecursiveControlType::StructLazy) => true,
            _ => false,
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
            _ => {
                panic!("parsing kind, {:?} not none, bool, f32, f32;2, s", value)
            }
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
            _ => {
                if value.starts_with("Lazy") {
                    HowToControlThis::WithRecurse(
                        OverrideOrInferred::Inferred,
                        RecursiveControlType::StructLazy,
                    )
                } else {
                    HowToControlThis::WithRecurse(
                        OverrideOrInferred::Inferred,
                        RecursiveControlType::Struct,
                    )
                }
            }
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
    fn from_control_type(&self, ty: ControlType, is_vec: bool) -> String {
        match (ty, self, is_vec) {
            (ControlType::F32, SerdeDefault::Zeros, _) => {
                "murrelet_livecode::livecode::_auto_default_f32_0".to_string()
            }
            (ControlType::F32, SerdeDefault::Ones, _) => {
                "murrelet_livecode::livecode::_auto_default_f32_1".to_string()
            }
            (ControlType::F32, SerdeDefault::CustomFunction(x), _) => x.clone(),
            (ControlType::Bool, SerdeDefault::Zeros, _) => {
                "murrelet_livecode::livecode::_auto_default_bool_false".to_string()
            }
            (ControlType::Bool, SerdeDefault::Ones, _) => {
                "murrelet_livecode::livecode::_auto_default_bool_true".to_string()
            }
            (ControlType::Bool, SerdeDefault::CustomFunction(x), _) => x.clone(),
            (ControlType::F32_2, SerdeDefault::Zeros, _) => {
                "murrelet_livecode::livecode::_auto_default_vec2_0".to_string()
            }
            (ControlType::F32_2, SerdeDefault::Ones, _) => {
                "murrelet_livecode::livecode::_auto_default_vec2_1".to_string()
            }
            (ControlType::F32_2, SerdeDefault::CustomFunction(x), _) => x.clone(),
            (ControlType::F32_3, SerdeDefault::CustomFunction(x), _) => x.clone(),

            (ControlType::Color, SerdeDefault::Zeros, _) => {
                "murrelet_livecode::livecode::_auto_default_color_0".to_string()
            }

            (ControlType::Color, SerdeDefault::Ones, _) => {
                "murrelet_livecode::livecode::_auto_default_color_1".to_string()
            }
            (ControlType::Color, SerdeDefault::CustomFunction(x), _) => x.clone(),
            (ControlType::ColorUnclamped, SerdeDefault::CustomFunction(x), _) => x.to_string(),

            (ControlType::LazyNodeF32, SerdeDefault::Zeros, false) => {
                "murrelet_livecode::livecode::_auto_default_f32_0_lazy".to_string()
            }
            (ControlType::LazyNodeF32, SerdeDefault::Ones, false) => {
                "murrelet_livecode::livecode::_auto_default_f32_1_lazy".to_string()
            }

            // handle Vec<LazyNodeF32>, which is what we use to represent things like Vec2
            (ControlType::LazyNodeF32, SerdeDefault::Zeros, true) => {
                "murrelet_livecode::livecode::_auto_default_f32_vec0_lazy".to_string()
            }
            (ControlType::LazyNodeF32, SerdeDefault::Ones, true) => {
                "murrelet_livecode::livecode::_auto_default_f32_vec1_lazy".to_string()
            }

            (ControlType::LazyNodeF32, SerdeDefault::CustomFunction(x), _) => x.clone(),

            _ => panic!(
                "serde default not implemented yet, need {:?} {:?}",
                ty, self
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DataFromType {
    pub(crate) main_type: syn::Ident,
    pub(crate) second_type: Option<syn::Ident>,
    pub(crate) third_type: Option<syn::Ident>, // so we coulddd use a vec her

    pub(crate) main_how_to: HowToControlThis,
    pub(crate) second_how_to: Option<HowToControlThis>,
    pub(crate) third_how_to: Option<HowToControlThis>, // so we coulddd use a vec her
}
impl DataFromType {
    fn new_from_list(types: Vec<syn::Ident>) -> DataFromType {
        assert!(!types.is_empty()); // should be by how it's programmed but...

        let main_type = types[0].clone();
        let second_type = types.get(1).cloned();
        let third_type = types.get(2).cloned();

        let main_how_to = HowToControlThis::from_type_str(&main_type.to_string());
        let second_how_to = second_type
            .as_ref()
            .map(|x| HowToControlThis::from_type_str(&x.to_string()));
        let third_how_to = third_type
            .as_ref()
            .map(|x| HowToControlThis::from_type_str(&x.to_string()));

        Self {
            main_type,
            second_type,
            third_type,
            main_how_to,
            second_how_to,
            third_how_to,
        }
    }

    pub(crate) fn how_to_control_internal(&self) -> &HowToControlThis {
        if let Some(third) = &self.third_how_to {
            third
        } else if let Some(second) = &self.second_how_to {
            second
        } else {
            &self.main_how_to
        }
    }

    pub(crate) fn internal_type(&self) -> syn::Ident {
        if let Some(third) = &self.third_type {
            third
        } else if let Some(second) = &self.second_type {
            second
        } else {
            &self.main_type
        }
        .clone()
    }

    pub(crate) fn wrapper_type(&self) -> VecDepth {
        match self.main_how_to {
            HowToControlThis::WithRecurse(_, RecursiveControlType::Vec) => {
                match self.second_how_to {
                    Some(HowToControlThis::WithRecurse(_, RecursiveControlType::Vec)) => {
                        VecDepth::VecVec
                    }
                    Some(_) => VecDepth::Vec,
                    None => unreachable!("vec should have a type??"),
                }
            }
            _ => VecDepth::NotAVec,
        }
    }
}

pub(crate) enum VecDepth {
    NotAVec,
    Vec,
    VecVec,
}

pub fn recursive_ident_from_path(t: &syn::Type, acc: &mut Vec<syn::Ident>) {
    match t {
        syn::Type::Path(syn::TypePath { path, .. }) => {
            let s = path.segments.last().unwrap();
            let main_type = s.ident.clone();

            acc.push(main_type);

            if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
                args,
                ..
            }) = s.arguments.clone()
            {
                if let syn::GenericArgument::Type(other_ty) = args.first().unwrap() {
                    recursive_ident_from_path(other_ty, acc);
                } else {
                    panic!("recursive ident not implemented yet {:?}", args);
                }
            }
        }
        x => panic!("no name for type {:?}", x),
    }
}

pub(crate) fn ident_from_type(t: &syn::Type) -> DataFromType {
    let mut acc = vec![];
    recursive_ident_from_path(t, &mut acc);

    // will always have at least one item
    DataFromType::new_from_list(acc)
}
