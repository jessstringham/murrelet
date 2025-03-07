use std::collections::HashMap;

use darling::{ast, FromDeriveInput, FromField, FromMeta, FromVariant};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

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
    fn from_type_struct(idents: StructIdents, how_to_control_this_type: &RandMethod) -> Self;
    fn from_type_recurse(
        idents: StructIdents,
        how_to_control_outer_type: &RandMethod,
        how_to_control_inner_type: &RandMethod,
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
                    HowToControlThis::Normal => panic!("should have an annotation"),
                    HowToControlThis::Type(how_to_control_this_type) => {
                        Self::from_type_struct(idents, &how_to_control_this_type)
                    }
                    HowToControlThis::Default => todo!(),
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
                    HowToControlThis::Type(how_to_control_this_type) => {
                        Self::from_type_struct(idents, &how_to_control_this_type)
                    }
                    HowToControlThis::Recurse(outer, inner) => {
                        Self::from_type_recurse(idents, &outer, &inner)
                    }
                    HowToControlThis::Override(func, count) => {
                        Self::from_override_struct(idents, &func, count)
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
    pub(crate) method: RandMethod,
    #[darling(default)]
    pub(crate) method_inner: Option<RandMethod>,
}
impl LivecodeFieldReceiver {
    fn how_to_control_this(&self) -> HowToControlThis {
        if let Some(OverrideFn { func, count }) = &self.override_fn {
            match func.as_str() {
                "default" => HowToControlThis::Default,
                _ => HowToControlThis::Override(func.clone(), *count),
            }
        } else if let Some(r) = &self.method_inner {
            HowToControlThis::Recurse(self.method.clone(), r.clone())
        } else if matches!(self.method, RandMethod::VecLength { .. }) {
            panic!("vec missing inner")
            // HowToControlThis::Recurse(self.method.clone(), None)
        } else {
            HowToControlThis::Type(self.method.clone())
        }
    }
}

#[derive(Debug, Clone, FromMeta)]
pub enum RandMethod {
    Recurse,
    BoolBinomial {
        pct: f32, // true
    },
    F32Uniform {
        start: syn::Expr,
        end: syn::Expr,
    },
    F32UniformPosNeg {
        start: f32,
        end: f32,
    },
    F32Normal {
        mu: syn::Expr,
        sigma: syn::Expr,
    },
    F32Fixed {
        val: syn::Expr,
    },
    Vec2UniformGrid {
        x: syn::Expr,
        y: syn::Expr,
        width: f32,
        height: f32,
    },
    Vec2Circle {
        x: syn::Expr,
        y: syn::Expr,
        radius: f32,
    },
    VecLength {
        min: usize,
        max: usize,
    },
    ColorNormal,
    ColorTransparency,
    StringChoice {
        choices: HashMap<String, f32>,
    },
}
impl RandMethod {
    pub(crate) fn to_methods(&self, ty: syn::Type, convert: bool) -> (TokenStream2, TokenStream2) {
        let maybe_as = if convert {
            quote! { as #ty }
        } else {
            quote! {}
        };

        match self {
            RandMethod::Recurse => {
                let for_rn_count = quote! { #ty::rn_count() };
                let for_make_gen = quote! {{
                    let r = #ty::sample_dist(rn, rn_start_idx);
                    rn_start_idx += #for_rn_count;
                    r
                }};

                (for_rn_count, for_make_gen)
            }
            RandMethod::BoolBinomial { pct } => {
                let for_rn_count = quote! { 1 };
                let for_make_gen = quote! { {
                    let result = rn[rn_start_idx] > #pct;
                    rn_start_idx += #for_rn_count;
                    result
                } };

                (for_rn_count, for_make_gen)
            }
            RandMethod::F32Uniform { start, end } => {
                let for_rn_count = quote! { 1 };
                let for_make_gen = quote! { {
                    let result = rn[rn_start_idx] * (#end - #start) + #start;
                    rn_start_idx += #for_rn_count;
                    result #maybe_as
                } };

                (for_rn_count, for_make_gen)
            }
            RandMethod::F32UniformPosNeg { start, end } => {
                let for_rn_count = quote! { 2 };
                let for_make_gen = quote! { {
                    let sgn = if(rn[rn_start_idx] > 0.5) { 1.0 } else { -1.0 };
                    let result = rn[rn_start_idx + 1] * (#end - #start) + #start;
                    rn_start_idx += #for_rn_count;
                    (sgn * result) #maybe_as
                } };

                (for_rn_count, for_make_gen)
            }
            RandMethod::F32Fixed { val } => (quote! { 0 }, quote! { #val  #maybe_as }),
            // box muller copy-pasta
            RandMethod::F32Normal { mu, sigma } => {
                let for_rn_count = quote! { 2 };
                let for_make_gen = quote! { {
                    // avoid nans, make sure u1 is positive and non-zero
                    let u1 = rn[rn_start_idx].clamp(std::f32::MIN_POSITIVE, 1.0);
                    let u2 = rn[rn_start_idx + 1].clamp(0.0, 1.0);
                    rn_start_idx += 2;

                    let r = (-2.0 * u1.ln()).sqrt();
                    let theta = 2.0 * std::f32::consts::PI * u2;

                    #mu + #sigma * r * theta.cos() #maybe_as
                } };
                (for_rn_count, for_make_gen)
            }

            RandMethod::Vec2UniformGrid {
                x,
                y,
                width,
                height,
            } => {
                let for_rn_count = quote! { 2 };
                let for_make_gen = quote! {{
                    let width = rn[rn_start_idx] * #width;
                    let height = rn[rn_start_idx + 1] * #height;

                    rn_start_idx += #for_rn_count;

                    glam::vec2(#x, #y) - 0.5 * glam::vec2(#width, #height) + glam::vec2(width, height)
                }};

                (for_rn_count, for_make_gen)
            }
            RandMethod::Vec2Circle { x, y, radius } => {
                let for_rn_count = quote! { 2 };

                let for_make_gen = quote! {{
                    let angle = rn[rn_start_idx] * 2.0 * std::f32::consts::PI;
                    let dist = rn[rn_start_idx + 1]; // sqrt it to even out the sampling
                    rn_start_idx += #for_rn_count;
                    glam::vec2(#x, #y) + glam::vec2(angle.cos(), angle.sin()) * #radius * dist.sqrt()
                }};

                (for_rn_count, for_make_gen)
            }
            RandMethod::ColorNormal => {
                let for_rn_count = quote! { 3 };

                let for_make_gen = quote! {{
                    let h = rn[rn_start_idx];
                    let s = rn[rn_start_idx + 1];
                    let v = rn[rn_start_idx + 2];
                    rn_start_idx += #for_rn_count;
                    murrelet_common::MurreletColor::hsva(h, s, v, 1.0)
                }};

                (for_rn_count, for_make_gen)
            }
            RandMethod::ColorTransparency => {
                let for_rn_count = quote! { 4 };

                let for_make_gen = quote! {{
                    let h = rn[rn_start_idx];
                    let s = rn[rn_start_idx + 1];
                    let v = rn[rn_start_idx + 2];
                    let a = rn[rn_start_idx + 3];
                    rn_start_idx += #for_rn_count;
                    murrelet_common::MurreletColor::hsva(h, s, v, a)
                }};

                (for_rn_count, for_make_gen)
            }
            RandMethod::VecLength { .. } => {
                unreachable!("hm, this should be in teh recurse func, are you missing an inner?")
            }
            RandMethod::StringChoice { choices } => {
                let one_hot = choices.len();
                let for_rn_count = quote! { #one_hot };

                let weighted_rns = choices
                    .iter()
                    .enumerate()
                    .map(|(i, (key, weight))| {
                        quote! { (#key.clone(), #weight * rn[rn_start_idx + #i]) }
                    })
                    .collect::<Vec<_>>();

                let for_make_gen = quote! { {
                    let result = vec![#(#weighted_rns,)*].into_iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).expect("empty string choices??");
                    rn_start_idx += #for_rn_count;
                    result.0.to_string()
                } };

                (for_rn_count, for_make_gen)
            }
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
    Normal,
    Type(RandMethod),
    Recurse(RandMethod, RandMethod), // one level... defaults to calling its func
    Default,                         // just do the default values
    Override(String, usize),
}
