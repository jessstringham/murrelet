use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::{gen_methods::GenMethod, parser::*};

pub(crate) struct FieldTokensGen {
    pub(crate) for_rn_count: TokenStream2,
    pub(crate) for_rn_names: TokenStream2,
    pub(crate) for_make_gen: TokenStream2,
    pub(crate) for_to_dist: TokenStream2,
    pub(crate) for_to_dist_mask: TokenStream2,
}
impl GenFinal for FieldTokensGen {
    // Something(f32)
    fn make_newtype_struct_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensGen>,
    ) -> TokenStream2 {
        let name = idents.name;

        let for_rn_count = variants.iter().map(|x| x.for_rn_count.clone());
        let for_rn_names = variants.iter().map(|x| x.for_rn_names.clone());
        let for_make_gen = variants.iter().map(|x| x.for_make_gen.clone());
        let for_to_dist = variants.iter().map(|x| x.for_to_dist.clone());
        let for_to_dist_mask = variants.iter().map(|x| x.for_to_dist_mask.clone());

        quote! {
            impl murrelet_gen::CanSampleFromDist for #name {
                fn rn_count() -> usize {
                    #(#for_rn_count+)*
                }

                fn rn_names() -> Vec<String> {
                    #(#for_rn_names+)*
                }

                fn sample_dist(rn: &[f32], start_idx: usize) -> Self {
                    Self(#(#for_make_gen,)*)
                }

                fn to_dist(&self) -> Vec<f32> {
                    let val = self;
                    vec![#(#for_to_dist,)*].concat()
                }

                fn to_dist_mask(&self) -> Vec<bool> {
                    let val = self;
                    vec![#(#for_to_dist_mask,)*].concat()
                }
            }
        }
    }

    fn make_struct_final(idents: ParsedFieldIdent, variants: Vec<FieldTokensGen>) -> TokenStream2 {
        let name = idents.name;

        let for_rn_count = variants.iter().map(|x| x.for_rn_count.clone());
        let for_rn_names = variants.iter().map(|x| x.for_rn_names.clone());
        let for_make_gen = variants.iter().map(|x| x.for_make_gen.clone());
        let for_to_dist = variants.iter().map(|x| x.for_to_dist.clone());
        let for_to_dist_mask = variants.iter().map(|x| x.for_to_dist_mask.clone());

        quote! {
            impl murrelet_gen::CanSampleFromDist for #name {
                fn rn_count() -> usize {
                    vec![
                        #(#for_rn_count,)*
                    ].iter().sum()
                }

                fn rn_names() -> Vec<String> {
                    vec![
                        #(#for_rn_names,)*
                    ].concat()
                }

                fn sample_dist(rn: &[f32], start_idx: usize) -> Self {
                    let mut rn_start_idx = start_idx;

                    Self {
                        #(#for_make_gen,)*
                    }
                }

                fn to_dist(&self) -> Vec<f32> {
                    let val = self;
                    vec![#(#for_to_dist,)*].concat()
                }

                fn to_dist_mask(&self) -> Vec<bool> {
                    let val = self;
                    vec![#(#for_to_dist_mask,)*].concat()
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
        let for_rn_names_all = variants.iter().map(|x| x.for_rn_names.clone());
        let for_to_dist = variants.iter().map(|x| x.for_to_dist.clone());
        let for_to_dist_mask = variants.iter().map(|x| x.for_to_dist_mask.clone());

        let mut weights = vec![];
        let mut for_rn_names: Vec<TokenStream2> = vec![];

        for ((variant, receiver), names) in variants
            .iter()
            .zip(variant_receiver.iter())
            .zip(for_rn_names_all)
        {
            let create_variant = &variant.for_make_gen;
            let receiver_name = receiver.ident.to_string();

            for_rn_names.push(quote! {
                murrelet_gen::prefix_field_names(#receiver_name.to_string(), vec![vec!["[weight]".to_string()], #names].concat())
            });

            let weight = receiver.weight;
            // hrm, might want to use closures if it's expensive
            // also the create_variant will modify rn_start_idx for us.
            weights.push(quote! {
                let weight = #weight * rn[rn_start_idx];
                rn_start_idx += 1;
                weighted_rns.push((weight, #create_variant));
            });
        }

        // one hot encoding, i might be off-by-one here for how many vars..
        let number_of_choices = variants.len();

        quote! {
            impl murrelet_gen::CanSampleFromDist for #name {
                // we add up each one individually, and then add one more for the type
                fn rn_count() -> usize {
                    vec![
                        #(#for_rn_count,)*
                    ].iter().sum::<usize>() + #number_of_choices
                }

                fn rn_names() -> Vec<String> {
                    vec![
                        #(#for_rn_names,)*
                    ].concat()
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

                fn to_dist(&self) -> Vec<f32> {
                    let val = self;
                    let mut result: Vec<f32> = vec![];
                    #(#for_to_dist;)*
                    result
                }

                fn to_dist_mask(&self) -> Vec<bool> {
                    let val = self;
                    let mut result: Vec<bool> = vec![];
                    #(#for_to_dist_mask;)*
                    result
                }
            }
        }
    }

    fn from_newtype_struct(idents: StructIdents, _parent_ident: syn::Ident) -> FieldTokensGen {
        let ty = convert_vec_type(&idents.data.ty);

        let for_rn_count = quote! { #ty::rn_count() };

        let for_rn_names = quote! { #ty::rn_names() };

        let for_make_gen = quote! {
            self.0.sample_dist(rn, rn_start_idx)
        };

        let for_to_dist = quote! {
            self.0.to_dist()
        };

        let for_to_dist_mask = quote! {
            self.0.to_dist_mask()
        };

        FieldTokensGen {
            for_rn_count,
            for_rn_names,
            for_make_gen,
            for_to_dist,
            for_to_dist_mask,
        }
    }

    // e.g. TileAxisLocs::V(TileAxisVs)
    fn from_unnamed_enum(idents: EnumIdents) -> FieldTokensGen {
        let variant_ident = idents.data.ident;
        let ty = convert_vec_type(&idents.data.fields.fields.first().unwrap().ty);
        let name = idents.enum_name;

        let for_rn_count = quote! { #ty::rn_count() };

        let for_rn_names = quote! { #ty::rn_names() };

        let for_to_dist = quote! {
            if let #name::#variant_ident(x) = &self {
                result.push(1.0);
                result.extend(x.to_dist().into_iter())
            } else {
                result.push(0.0);
                result.extend((0..#ty::rn_count()).map(|x| 0.5));
            }
        };

        let for_to_dist_mask = quote! {
            if let #name::#variant_ident(x) = &self {
                result.push(true);
                result.extend((0..#ty::rn_count()).map(|x| true));
            } else {
                result.push(true);
                result.extend((0..#ty::rn_count()).map(|x| false));
            }
        };

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
            for_rn_names,
            for_make_gen,
            for_to_dist,
            for_to_dist_mask,
        }
    }

    // e.g. TileAxis::Diag
    fn from_unit_enum(idents: EnumIdents) -> FieldTokensGen {
        let variant_ident = idents.data.ident;
        let name = idents.enum_name;

        // just the one-hot encoding
        let for_rn_count = quote! { 0 };

        let for_rn_names = quote! { vec![] };

        let for_make_gen = quote! { #name::#variant_ident };

        let for_to_dist = quote! {
           if let #name::#variant_ident = &self {
               result.push(1.0);
           } else {
               result.push(0.0);
           }
        };

        let for_to_dist_mask = quote! {
           if let #name::#variant_ident = &self {
               result.push(true);
           } else {
               result.push(true);
           }
        };

        FieldTokensGen {
            for_rn_count,
            for_rn_names,
            for_make_gen,
            for_to_dist,
            for_to_dist_mask,
        }
    }

    // // skip
    // fn from_noop_struct(idents: StructIdents) -> FieldTokensGen {
    //     let field_name = idents.data.ident.unwrap().to_string();
    //     let ty = idents.data.ty;

    //     let for_rn_count = quote! { 0 };
    //     let for_rn_names = quote! { vec![] };
    //     let for_make_gen = quote! { #field_name: #ty::default() };
    //     let for_to_dist = quote! { vec![] };
    //     let for_to_dist_mask = quote! { vec![] };

    //     FieldTokensGen {
    //         for_rn_count,
    //         for_make_gen,
    //         for_rn_names,
    //         for_to_dist,
    //         for_to_dist_mask,
    //     }
    // }

    // f32, Vec2, etc
    fn from_type_struct(idents: StructIdents, method: &GenMethod) -> FieldTokensGen {
        let field_name = idents.data.ident.unwrap();
        let field_name_str = field_name.to_string();
        let ty = idents.data.ty;

        let (for_rn_count, for_rn_names, for_make_gen, for_to_dist) =
            method.to_methods(ty, quote! {self.#field_name}, true);

        // for the mask. if we're here, we probably want to count it!
        let rn_count = for_rn_count.clone();

        FieldTokensGen {
            for_make_gen: quote! { #field_name: #for_make_gen },
            for_rn_names: quote! { murrelet_gen::prefix_field_names(#field_name_str.to_string(), #for_rn_names)},
            for_rn_count,
            for_to_dist,
            for_to_dist_mask: quote! { (0..{#rn_count}).into_iter().map(|x| true).collect::<Vec<_>>() },
        }
    }

    fn from_type_recurse(idents: StructIdents, outer: &GenMethod, inner: &GenMethod) -> Self {
        let field_name = idents.data.ident.unwrap();
        let ty = idents.data.ty;

        let (for_rn_count, for_rn_names, for_make_gen, for_to_dist, for_to_dist_mask) = match outer
        {
            GenMethod::VecLength { min, max } => {
                let inside_type = nested_ident(&ty);

                let i = inside_type[1].clone();
                let inside_type_val: syn::Type = syn::parse_quote! { #i };

                let (
                    for_rn_count_per_item,
                    for_rn_names_per_item,
                    for_make_gen_per_item,
                    for_make_to_dist_per_item,
                ) = inner.to_methods(inside_type_val, quote! {val.clone()}, false);

                let for_rn_count: TokenStream2 = quote! {
                    #for_rn_count_per_item * #max + 1
                };

                let for_rn_names_all = (0..*max).into_iter().map(|x| {
                    let i_name = x.to_string();
                    quote! { murrelet_gen::prefix_field_names(#i_name.to_string(), #for_rn_names_per_item) }
                });

                let field_name_str = field_name.to_string();

                let for_rn_names = quote! {
                    murrelet_gen::prefix_field_names(
                        #field_name_str.to_string(),
                        vec![
                            vec!["[len]".to_string()],
                           #(#for_rn_names_all,)*
                        ].concat()
                    )
                };

                // in this case, we _don't_ want one-hot, because it actually does make
                // sense to interpolate between say, 3 and 6.
                // i want to add extra indicators for everything between min and max
                // but i'm not sure how to do that! because i'm just generating,
                // not making the input data for something else...
                let for_make_gen = quote! {{
                    let range = (#max - #min + 1) as f32;
                    let how_many = (rn[rn_start_idx] * range) as usize + #min;
                    rn_start_idx += 1;
                    let mut v = vec![];
                    // need to go through the fill list so we increment
                    // through the rns right
                    for i in 0..#max {
                        if i < how_many {
                            v.push(#for_make_gen_per_item);
                        } else {
                            // just run it
                            #for_make_gen_per_item;
                        }
                    }
                    v
                }};

                let for_to_dist = quote! {{
                    let mut result = vec![];
                    let x = self.#field_name.len() as f32;
                    let v = (x - #min as f32) / ((#max - #min) as f32);
                    result.push(v);
                    for val in self.#field_name.iter() {
                        // always extend it
                        let vv = #for_make_to_dist_per_item;
                        result.extend(vv.into_iter());
                    }

                    for _ in self.#field_name.len()..#max {
                        let vv: Vec<f32> = (0..#for_rn_count_per_item).into_iter().map(|_| 0.5f32).collect();
                        result.extend(vv.into_iter());
                    }
                    result
                }};

                let for_to_dist_mask = quote! {{
                    let mut result: Vec<bool> = vec![];
                    result.push(true);
                    for val in self.#field_name.iter() {
                        // always extend it
                        let vv: Vec<bool> = (0..#for_rn_count_per_item).into_iter().map(|_| true).collect();
                        result.extend(vv.into_iter());
                    }

                    for _ in self.#field_name.len()..#max {
                        // these ones are all false!
                        let vv: Vec<bool> = (0..#for_rn_count_per_item).into_iter().map(|_| false).collect();
                        result.extend(vv.into_iter());
                    }
                    result
                }};

                (
                    for_rn_count,
                    for_rn_names,
                    for_make_gen,
                    for_to_dist,
                    for_to_dist_mask,
                )
            }
            _ => unreachable!("not expecting an inner without a recursive outer"),
        };

        Self {
            for_rn_count,
            for_rn_names,
            for_make_gen: quote! { #field_name: #for_make_gen },
            for_to_dist,
            for_to_dist_mask,
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
