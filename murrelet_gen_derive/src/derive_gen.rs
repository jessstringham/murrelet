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
                    #(#for_rn_count+)*
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

        for (variant, receiver) in variants.iter().zip(variant_receiver.iter()) {
            let create_variant = &variant.for_make_gen;
            let rn_gen = &variant.for_rn_count;

            let weight = receiver.weight;
            // we need the closures so we offset it right... hrm, shadowign the variable, mgiht regret that
            weights.push(quote! {
                let weight = #weight * rn[rn_start_idx];
                rn_start_idx += 1;
                weighted_rns.push((weight, #create_variant));
                rn_start_idx += #rn_gen;
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
                    ].iter().sum::<usize>() + #number_of_choices
                }

                fn sample_dist(rn: &[f32], start_idx: usize) -> Self {
                    let mut rn_start_idx = start_idx;

                    let mut weighted_rns: Vec<(f32, _)> = vec![];
                    #(#weights;)*

                    // first choose which enum
                    let (_, comp) = weighted_rns.into_iter()
                        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
                        .expect("no enum values?");

                    comp
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
            self.0.sample_dist(rn, rn_start_idx)
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

        // hm, i'm not sure that the method in the enum is actually used
        let for_make_gen = quote! {
             {
                let result = #name::#variant_ident(#ty::sample_dist(rn, rn_start_idx));
                rn_start_idx += #for_rn_count;
                result

            }
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
    fn from_type_struct(idents: StructIdents, method: &RandMethod) -> FieldTokensGen {
        let field_name = idents.data.ident.unwrap();
        let ty = idents.data.ty;

        let (for_rn_count, for_make_gen) = method.to_methods(ty, true);

        FieldTokensGen {
            for_make_gen: quote! { #field_name: #for_make_gen },
            for_rn_count,
        }
    }

    fn from_type_recurse(idents: StructIdents, outer: &RandMethod, inner: &RandMethod) -> Self {
        let field_name = idents.data.ident.unwrap();
        let ty = idents.data.ty;

        let (for_rn_count, for_make_gen) = match outer {
            RandMethod::VecLength { min, max } => {
                let inside_type = nested_ident(&ty);

                let i = inside_type[1].clone();
                let inside_type_val: syn::Type = syn::parse_quote! { #i };

                // (for_rn_count, for_make_gen)
                let (for_rn_count_per_item, for_make_gen_per_item) =
                    inner.to_methods(inside_type_val, false);

                let for_rn_count = quote! {
                    #for_rn_count_per_item * #max + 1
                };

                // in this case, we _don't_ want one-hot, because it actually does make
                // sense to interpolate between say, 3 and 6.
                // i want to add extra indicators for everything between min and max
                // but i'm not sure how to do that! because i'm just generating,
                // not making the input data for something else...
                let for_make_gen = quote! {{
                    let range = (#max - #min) as f32;
                    let how_many = (rn[rn_start_idx] * range) as usize + #min;
                    rn_start_idx += 1;
                    let mut v = vec![];
                    for _ in 0..how_many {
                        v.push(#for_make_gen_per_item);
                    }
                    v
                }};

                (for_rn_count, for_make_gen)
            }
            _ => unreachable!("not expecting an inner without a recursive outer"),
        };

        Self {
            for_rn_count,
            for_make_gen: quote! { #field_name: #for_make_gen },
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
