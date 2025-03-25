use darling::FromMeta;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use std::collections::HashMap;

#[derive(Debug, Clone, FromMeta)]
pub enum GenMethod {
    Default,
    Recurse,
    BoolBinomial {
        pct: f32, // pct that is true
    },
    F32Uniform {
        start: syn::Expr,
        end: syn::Expr,
    },
    F32UniformPosNeg {
        // includes between start and end as negative and positive
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
        // chooses random points
        x: syn::Expr,
        y: syn::Expr,
        width: f32,
        height: f32,
    },
    Vec2Circle {
        // selects random points within a circle
        x: syn::Expr,
        y: syn::Expr,
        radius: f32,
    },
    VecLength {
        // determines how long the vector will be
        min: usize,
        max: usize,
    },
    ColorNormal,       // samples h, s, and v values
    ColorTransparency, // same as ColorNormal, plus alpha
    StringChoice {
        choices: HashMap<String, f32>, // string mapped to its weight
    },
}
impl GenMethod {
    pub(crate) fn to_methods(
        &self,
        ty: syn::Type,
        name: TokenStream2,
        convert: bool,
    ) -> (TokenStream2, TokenStream2, TokenStream2, TokenStream2) {
        let maybe_as = if convert {
            quote! { as #ty }
        } else {
            quote! {}
        };

        match self {
            GenMethod::Recurse => {
                let for_rn_count = quote! { #ty::rn_count() };
                let for_rn_names = quote! { #ty::rn_names() };
                let for_make_gen = quote! {{
                    let r = #ty::sample_dist(rn, rn_start_idx);
                    rn_start_idx += #for_rn_count;
                    r
                }};
                let for_to_dist = quote! { #name.to_dist() };

                (for_rn_count, for_rn_names, for_make_gen, for_to_dist)
            }
            GenMethod::BoolBinomial { pct } => {
                let for_rn_count = quote! { 1 };
                let for_rn_names = quote! { vec!["pct".to_string()] };
                let for_make_gen = quote! { {
                    let result = rn[rn_start_idx] > #pct;
                    rn_start_idx += #for_rn_count;
                    result
                } };
                let for_to_dist = quote! {
                    if #name {
                        vec![1.0]
                    } else {
                        vec![0.0]
                    }
                };

                (for_rn_count, for_rn_names, for_make_gen, for_to_dist)
            }
            GenMethod::F32Uniform { start, end } => {
                let for_rn_count = quote! { 1 };
                let for_rn_names = quote! { vec!["uniform".to_string()] };
                let for_make_gen = quote! { {
                    let result = rn[rn_start_idx] * (#end - #start) + #start;
                    rn_start_idx += #for_rn_count;
                    result #maybe_as
                } };
                let for_to_dist = quote! { vec![(#name as f32 - #start) / (#end - #start)] };

                (for_rn_count, for_rn_names, for_make_gen, for_to_dist)
            }
            GenMethod::F32UniformPosNeg { start, end } => {
                let for_rn_count = quote! { 2 };
                let for_rn_names = quote! { vec!["uniform".to_string(), "sign".to_string()] };
                let for_make_gen = quote! { {
                    let sgn = if(rn[rn_start_idx] > 0.5) { 1.0 } else { -1.0 };
                    let result = rn[rn_start_idx + 1] * (#end - #start) + #start;
                    rn_start_idx += #for_rn_count;
                    (sgn * result) #maybe_as
                } };

                let for_to_dist = quote! { vec![
                    if #name > 0.0 { 1.0 } else { 0.0 },
                    (#name.abs() - #start) / (#end - #start)
                ] };

                (for_rn_count, for_rn_names, for_make_gen, for_to_dist)
            }
            GenMethod::F32Fixed { val } => (
                quote! { 0 },
                quote! {vec![]},
                quote! { #val #maybe_as },
                quote! {vec![]},
            ),
            GenMethod::F32Normal { mu, sigma } => {
                let for_rn_count = quote! { 2 };
                let for_rn_names =
                    quote! { vec![ "BoxMuller1".to_string(), "BoxMuller2".to_string()] };
                let for_make_gen = quote! { {
                    // avoid nans, make sure u1 is positive and non-zero
                    let u1 = rn[rn_start_idx].clamp(std::f32::MIN_POSITIVE, 1.0);
                    let u2 = rn[rn_start_idx + 1].clamp(0.0, 1.0);
                    rn_start_idx += 2;

                    let r = (-2.0 * u1.ln()).sqrt();
                    let theta = 2.0 * std::f32::consts::PI * u2;

                    #mu + #sigma * r * theta.cos() #maybe_as
                } };
                let for_to_dist = quote! { todo!() };
                (for_rn_count, for_rn_names, for_make_gen, for_to_dist)
            }
            GenMethod::Vec2UniformGrid {
                x,
                y,
                width,
                height,
            } => {
                let for_rn_count = quote! { 2 };
                let for_rn_names = quote! { vec![ "x".to_string(), "y".to_string()] };
                let for_make_gen = quote! {{
                    let width = rn[rn_start_idx] * #width;
                    let height = rn[rn_start_idx + 1] * #height;

                    rn_start_idx += #for_rn_count;

                    glam::vec2(#x, #y) - 0.5 * glam::vec2(#width, #height) + glam::vec2(width, height)
                }};

                let for_to_dist = quote! { {
                    let c = (#name + 0.5 * glam::vec2(#width, #height));
                    vec![c.x / #width, c.y / #height]
                }};

                (for_rn_count, for_rn_names, for_make_gen, for_to_dist)
            }
            GenMethod::Vec2Circle { x, y, radius } => {
                let for_rn_count = quote! { 2 };
                let for_rn_names = quote! { vec![ "theta".to_string(), "rad".to_string()] };

                let for_make_gen = quote! {{
                    let angle = rn[rn_start_idx] * 2.0 * std::f32::consts::PI;
                    let dist = rn[rn_start_idx + 1]; // sqrt it to even out the sampling
                    rn_start_idx += #for_rn_count;
                    glam::vec2(#x, #y) + glam::vec2(angle.cos(), angle.sin()) * #radius * dist.sqrt()
                }};

                let for_to_dist = quote! { {
                    let c = #name - glam::vec2(#x, #y);
                    let dist = (c.length() / #radius).powi(2);
                    let mut angle = c.to_angle();
                    if angle <= 0.0 {
                        angle += 2.0 * std::f32::consts::PI
                    }
                    angle /= (2.0 * std::f32::consts::PI);
                    vec![angle, dist]
                }};

                (for_rn_count, for_rn_names, for_make_gen, for_to_dist)
            }
            GenMethod::ColorNormal => {
                let for_rn_count = quote! { 3 };

                let for_rn_names =
                    quote! { vec![ "hue".to_string(), "sat".to_string(), "val".to_string()] };

                let for_make_gen = quote! {{
                    let h = rn[rn_start_idx];
                    let s = rn[rn_start_idx + 1];
                    let v = rn[rn_start_idx + 2];
                    rn_start_idx += #for_rn_count;
                    murrelet_common::MurreletColor::hsva(h, s, v, 1.0)
                }};

                let for_to_dist = quote! {{
                    let [h, s, v, _] = #name.into_hsva_components();
                    vec![
                        h % 1.0,
                        s % 1.0,
                        v % 1.0,
                    ]
                }};

                (for_rn_count, for_rn_names, for_make_gen, for_to_dist)
            }
            GenMethod::ColorTransparency => {
                let for_rn_count = quote! { 4 };

                let for_rn_names = quote! { vec![ "hue".to_string(), "sat".to_string(), "val".to_string(), "alpha".to_string()] };

                let for_make_gen = quote! {
                    {
                        let h = rn[rn_start_idx];
                        let s = rn[rn_start_idx + 1];
                        let v = rn[rn_start_idx + 2];
                        let a = rn[rn_start_idx + 3];
                        rn_start_idx += #for_rn_count;
                        murrelet_common::MurreletColor::hsva(h, s, v, a)
                    }
                };

                let for_to_dist = quote! { {
                    let [h, s, v, a] = #name.into_hsva_components();
                    vec![
                        h % 1.0,
                        s % 1.0,
                        v % 1.0,
                        a % 1.0,
                    ]
                } };

                (for_rn_count, for_rn_names, for_make_gen, for_to_dist)
            }
            GenMethod::VecLength { .. } => {
                // this is handled in the vec parser
                unreachable!("this location of veclength isn't supported yet!")
            }
            GenMethod::StringChoice { choices } => {
                let one_hot = choices.len();
                let for_rn_count = quote! { #one_hot };

                // let for_rn_names = quote! { vec![ "hue", "sat", "val", "alpha"] };
                let rn_names = choices.iter().map(|(key, _)| quote! { #key.to_string() });
                let for_rn_names = quote! { vec![#(#rn_names,)*] };

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

                let for_to_dist_choices = choices
                .iter()
                .map(|(key, _)| {
                    quote! { if #key.clone() == #name { result.push(1.0) } else { result.push(0.0) } }
                })
                .collect::<Vec<_>>();

                let for_to_dist = quote! { {
                    let mut result = vec![];
                    #(#for_to_dist_choices;)*
                    result
                } };

                (for_rn_count, for_rn_names, for_make_gen, for_to_dist)
            }
            GenMethod::Default => {
                let for_rn_count = quote! { 0 };

                let for_rn_names = quote! { vec![] };

                let for_make_gen = quote! { {
                    Default::default()
                } };

                let for_to_dist = quote! { vec![] };

                (for_rn_count, for_rn_names, for_make_gen, for_to_dist)
            }
        }
    }

    // pub(crate) fn from_methods(&self) -> TokenStream2 {
    //     match self {
    //         GenMethod::Default => todo!(),
    //         GenMethod::Recurse => todo!(),
    //         GenMethod::BoolBinomial { pct } => {
    //             // put at extremes

    //         },
    //         GenMethod::F32Uniform { start, end } => {

    //         },

    //     }
    // }
}
