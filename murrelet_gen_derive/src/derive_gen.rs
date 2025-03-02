use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::parser::*;

pub(crate) struct FieldTokensGen {
    pub(crate) for_rn_count: TokenStream2,
    pub(crate) for_make_gen: TokenStream2,
}
impl GenFinal for FieldTokensGen {
    // Something(f32)
    fn make_newtype_struct_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensGen>,
    ) -> TokenStream2 {
        let name = idents.name;

        let for_rn_count = variants.iter().map(|x| x.for_rn_count.clone());
        let for_make_gen = variants.iter().map(|x| x.for_make_gen.clone());

        quote! {
            impl murrelet_gen::CanSampleFromDist for #name {
                fn rn_count() -> usize {
                    vec![
                        #(#for_rn_count,)*
                    ].iter().sum()
                }

                fn sample_dist(rn: &[f32], start_idx: usize) -> Self {
                    Self(#(#for_make_gen,)*)
                }
            }
        }
    }

    fn make_struct_final(idents: ParsedFieldIdent, variants: Vec<FieldTokensGen>) -> TokenStream2 {
        let name = idents.name;

        let for_rn_count = variants.iter().map(|x| x.for_rn_count.clone());
        let for_make_gen = variants.iter().map(|x| x.for_make_gen.clone());

        quote! {
            impl murrelet_gen::CanSampleFromDist for #name {
                fn rn_count() -> usize {
                    vec![
                        #(#for_rn_count,)*
                    ].iter().sum()
                }

                fn sample_dist(rn: &[f32], start_idx: usize) -> Self {
                    let mut rn_start_idx = start_idx;
                    Self {
                        #(#for_make_gen,)*
                    }
                }
            }
        }
    }

    fn make_enum_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensGen>,
        variant_receiver: &[LivecodeVariantReceiver],
    ) -> TokenStream2 {
        let name = idents.name;
        let for_rn_count = variants.iter().map(|x| x.for_rn_count.clone());

        // let mut cumulative_probabilities = vec![];

        // let mut running_total = 0.0;
        // for receiver in variant_receiver.iter() {
        //     let weight = receiver.weight;
        //     running_total += weight;
        //     cumulative_probabilities.push(running_total);
        // }

        // // normalize
        // for i in 0..cumulative_probabilities.len() {
        //     cumulative_probabilities[i] /= running_total;
        // }

        // let mut q = vec![];
        // for (i, variant) in variants.iter().enumerate() {
        //     let create_variant = &variant.for_make_gen;
        //     let prob_upper_bound = cumulative_probabilities[i];
        //     q.push(quote! {
        //         x if x < #prob_upper_bound => #create_variant
        //     });
        // }

        let mut weights = vec![];
        let mut comps = vec![];
        for (i, (variant, receiver)) in variants.iter().zip(variant_receiver.iter()).enumerate() {
            let create_variant = &variant.for_make_gen;
            let rn_gen = &variant.for_rn_count;

            let weight = receiver.weight;
            weights.push(quote! {#weight * rn[rn_start_idx + #i]});

            // hm, if this turns out slow, look into closures
            comps.push(quote! {
                (#rn_gen, #create_variant)
            });
        }

        // one hot encoding
        let number_of_choices = variants.len();

        quote! {
            impl murrelet_gen::CanSampleFromDist for #name {
                // we add up each one individually, and then add one more for the type
                fn rn_count() -> usize {
                    vec![
                        #(#for_rn_count,)*
                    ].iter().sum() + #number_of_choices
                }

                fn sample_dist(rn: &[f32], start_idx: usize) -> Self {
                    let mut rn_start_idx = start_idx;

                    let weighted_rns = vec![#(#weights,)*];

                    // first choose which enum
                    if let Some((max_idx, max_val)) = weighted_rns.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()) {
                        println!("Max value {} is at index {}", max_val, max_idx);
                    } else {
                        unimplemented!("hrmrm, empty enum?")
                    }

                    rn_start_idx += #number_of_choices;

                    for (i, (rn_offset, comp)) in vec![#(#comps,)*].into_iter().enumerate() {
                        if i == max_idx {
                            return comp
                        } else {
                            rn_start_idx += rn_offset;
                        }
                    }

                    // match enum_rn {
                    //     #(#q,)*
                    // }
                }
            }
        }
    }

    fn from_override_enum(func: &str, rn_count: usize) -> FieldTokensGen {
        let method: syn::Path = syn::parse_str(func).expect("Custom method is invalid path!");

        let for_rn_count = quote! { #rn_count };

        let for_make_gen = quote! {
            #method()
        };

        FieldTokensGen {
            for_rn_count,
            for_make_gen,
        }
    }

    fn from_newtype_struct(idents: StructIdents, _parent_ident: syn::Ident) -> FieldTokensGen {
        let ty = convert_vec_type(&idents.data.ty);

        let for_rn_count = quote! { #ty::rn_count() };

        let for_make_gen = quote! {
            self.0.sample_dist(rng)
        };

        FieldTokensGen {
            for_rn_count,
            for_make_gen,
        }
    }

    // e.g. TileAxisLocs::V(TileAxisVs)
    fn from_unnamed_enum(idents: EnumIdents) -> FieldTokensGen {
        let variant_ident = idents.data.ident;
        let ty = convert_vec_type(&idents.data.fields.fields.first().unwrap().ty);
        let name = idents.enum_name;

        let for_rn_count = quote! { #ty::rn_count() };

        let for_make_gen = quote! {
            quote! { #name::#variant_ident(s.sample_dist()) };
        };

        FieldTokensGen {
            for_rn_count,
            for_make_gen,
        }
    }

    // e.g. TileAxis::Diag
    fn from_unit_enum(idents: EnumIdents) -> FieldTokensGen {
        let variant_ident = idents.data.ident;
        let name = idents.enum_name;

        // just the one-hot encoding
        let for_rn_count = quote! { 0 };

        let for_make_gen = quote! { #name::#variant_ident };

        FieldTokensGen {
            for_rn_count,
            for_make_gen,
        }
    }

    // skip
    fn from_noop_struct(idents: StructIdents) -> FieldTokensGen {
        let field_name = idents.data.ident.unwrap().to_string();
        let ty = idents.data.ty;

        let for_rn_count = quote! { 0 };
        let for_make_gen = quote! { #field_name: #ty::default() };

        FieldTokensGen {
            for_rn_count,
            for_make_gen,
        }
    }

    // f32, Vec2, etc
    fn from_type_struct(idents: StructIdents, htctt: &HowToControlThisType) -> FieldTokensGen {
        let field_name = idents.data.ident.unwrap();
        let ty = idents.data.ty;

        let (for_rn_count, for_make_gen) = match htctt {
            HowToControlThisType::Bool(rand_method_bool) => match rand_method_bool {
                RandMethodBool::Binomial { pct } => {
                    let for_rn_count = quote! { 1 };
                    let for_make_gen = quote! { {
                        let result = rn[rn_start_idx] > #pct;
                        rn_start_idx += #for_rn_count;
                        result
                    } };

                    (for_rn_count, for_make_gen)
                }
            },
            HowToControlThisType::F32(rand_method_f32) => match rand_method_f32 {
                RandMethodF32::Uniform { start, end } => {
                    let for_rn_count = quote! { 1 };
                    let for_make_gen = quote! { {
                        let result = rn[rn_start_idx] * (#end - #start) + #start;
                        rn_start_idx += #for_rn_count;
                        result as #ty
                    } };

                    (for_rn_count, for_make_gen)
                }
            },
            HowToControlThisType::Vec2(rand_method_vec2) => {
                match rand_method_vec2 {
                    RandMethodVec2::UniformGrid {
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
                    RandMethodVec2::Circle { x, y, radius } => {
                        let for_rn_count = quote! { 2 };

                        let for_make_gen = quote! {{
                            let angle = rn[rn_start_idx] * 2.0 * std::f32::consts::PI;
                            let dist = rn[rn_start_idx + 1]; // sqrt it to even out the sampling
                            rn_start_idx += #for_rn_count;
                            glam::vec2(#x, #y) + glam::vec2(angle.cos(), angle.sin()) * #radius * dist.sqrt()
                        }};

                        (for_rn_count, for_make_gen)
                    }
                }
            }
            HowToControlThisType::Vec(rand_method_vec) => match rand_method_vec {
                RandMethodVec::Length { min, max } => {
                    let inside_type = nested_ident(&ty);

                    let i = inside_type[1].clone();

                    let for_rn_count = quote! {
                        #i::rn_count() * #max + 1
                    };
                    // // in this case, we _don't_ want one-hot, because it actually does make
                    // // sense to interpolate between say, 3 and 6.
                    // // i want to add extra indicators for everything between min and max
                    // // but i'm not sure how to do that! because i'm just generating,
                    // // not making the input data for something else...
                    // let for_make_gen = quote! {{
                    //     let how_many = rn[rn_start_idx] * (#max - #min) + #min as usize;

                    //     rn_start_idx += 1;

                    //     let mut v = vec![];
                    //     for _ in 0..how_many {
                    //         v.push(#i::sample_dist(rn))
                    //         rn_start_idx += #i::rn_count();
                    //     }

                    //     v
                    // }};

//                     let for_rn_count = quote! {
// 1
//                     };

                    let for_make_gen = quote! {
                        vec![1.0]
                    };



                    (for_rn_count, for_make_gen)
                }
            },
            HowToControlThisType::Color(rand_method_color) => match rand_method_color {
                RandMethodColor::Normal => {
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
                RandMethodColor::Transparency => {
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
            },
        };

        FieldTokensGen {
            for_make_gen: quote! { #field_name: #for_make_gen },
            for_rn_count,
        }
    }

    fn from_override_struct(idents: StructIdents, func: &str, rn_count: usize) -> FieldTokensGen {
        let field_name = idents.data.ident.unwrap();

        let for_rn_count = quote! { #rn_count };

        let method: syn::Path = syn::parse_str(func).expect("Custom method is invalid path!");

        let for_make_gen = quote! { #field_name: #method() };

        FieldTokensGen {
            for_rn_count,
            for_make_gen,
        }
    }
}

fn recursive_ident_from_path(t: &syn::Type, acc: &mut Vec<syn::Ident>) {
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

fn nested_ident(t: &syn::Type) -> Vec<syn::Ident> {
    let mut acc = vec![];
    recursive_ident_from_path(t, &mut acc);
    return acc;
}

// we need to use turbofish to call an associated function
fn convert_vec_type(ty: &syn::Type) -> TokenStream2 {
    if let syn::Type::Path(type_path) = ty {
        if let Some(last_segment) = type_path.path.segments.last() {
            if last_segment.ident == "Vec" {
                if let syn::PathArguments::AngleBracketed(angle_bracketed) = &last_segment.arguments
                {
                    if let Some(inner_arg) = angle_bracketed.args.first() {
                        return quote! { Vec:: < #inner_arg > };
                    }
                }
            }
        }
    }

    quote! { #ty }
}
