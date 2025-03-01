use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::parser::*;

pub(crate) struct FieldTokensGen {
    pub(crate) for_make_gen: TokenStream2,
}
impl GenFinal for FieldTokensGen {
    // Something(f32)
    fn make_newtype_struct_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensGen>,
    ) -> TokenStream2 {
        let name = idents.name;

        let for_make_gen = variants.iter().map(|x| x.for_make_gen.clone());

        quote! {
            impl murrelet_gen::CanSampleFromDist for #name {
                fn sample_dist<R: rand::Rng>(rng: &mut R) -> Self {
                    Self(#(#for_make_gen,)*)
                }
            }
        }
    }

    fn make_struct_final(idents: ParsedFieldIdent, variants: Vec<FieldTokensGen>) -> TokenStream2 {
        let name = idents.name;

        let for_make_gen = variants.iter().map(|x| x.for_make_gen.clone());

        quote! {
            impl murrelet_gen::CanSampleFromDist for #name {
                fn sample_dist<R: rand::Rng>(rng: &mut R) -> Self {
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

        let mut cumulative_probabilities = vec![];

        let mut running_total = 0.0;
        for receiver in variant_receiver.iter() {
            let weight = receiver.weight;
            running_total += weight;
            cumulative_probabilities.push(running_total);
        }

        // normalize
        for i in 0..cumulative_probabilities.len() {
            cumulative_probabilities[i] /= running_total;
        }

        let mut q = vec![];
        for (i, variant) in variants.iter().enumerate() {
            let create_variant = &variant.for_make_gen;
            let prob_upper_bound = cumulative_probabilities[i];
            q.push(quote! {
                x if x < #prob_upper_bound => #create_variant
            });
        }

        quote! {
            impl murrelet_gen::CanSampleFromDist for #name {
                fn gen<R: rand::Rng>(rng: &mut R) -> Self {
                    // first choose which enum
                    let enum_rn = rng.gen();
                    match enum_rn {
                        #(#q,)*
                    }
                }
            }
        }
    }

    fn from_override_enum(func: &str) -> FieldTokensGen {
        let method: syn::Path = syn::parse_str(func).expect("Custom method is invalid path!");

        let for_make_gen = quote! {
            #method()
        };

        FieldTokensGen { for_make_gen }
    }

    fn from_newtype_struct(_idents: StructIdents, _parent_ident: syn::Ident) -> FieldTokensGen {
        let for_make_gen = quote! {
            self.0.sample_dist(rng)
        };

        FieldTokensGen { for_make_gen }
    }

    // e.g. TileAxisLocs::V(TileAxisVs)
    fn from_unnamed_enum(idents: EnumIdents) -> FieldTokensGen {
        let variant_ident = idents.data.ident;
        let name = idents.enum_name;

        let for_make_gen = quote! {
            quote! { #name::#variant_ident(s.sample_dist()) };
        };

        FieldTokensGen { for_make_gen }
    }

    // e.g. TileAxis::Diag
    fn from_unit_enum(idents: EnumIdents) -> FieldTokensGen {
        let variant_ident = idents.data.ident;
        let variant_ident_str = variant_ident.to_string();

        let for_make_gen =
            quote! { murrelet_gui::MurreletEnumValGUI::Unit(#variant_ident_str.to_owned()) };
        // let for_gui_to_livecode =
        //     quote! { murrelet_gui::Unit(#variant_ident_str) => #name::#variant_ident };

        FieldTokensGen {
            for_make_gen,
            // for_gui_to_livecode,
            // for_assign_vars: quote!(),
        }
    }

    // skip
    fn from_noop_struct(idents: StructIdents) -> FieldTokensGen {
        let field_name = idents.data.ident.unwrap().to_string();

        let for_make_gen =
            quote! { v.push((#field_name.to_owned(), murrelet_gui::MurreletGUISchema::Skip)) };
        // let for_gui_to_livecode =
        //     quote! { murrelet_gui::Unit(#variant_ident_str) => #name::#variant_ident };

        FieldTokensGen {
            for_make_gen,
            // for_gui_to_livecode,
        }
    }

    // f32, Vec2, etc
    fn from_type_struct(idents: StructIdents, htctt: &HowToControlThisType) -> FieldTokensGen {
        let field_name = idents.data.ident.unwrap();

        let for_make_gen = match htctt {
            HowToControlThisType::Bool(rand_method_bool) => match rand_method_bool {
                RandMethodBool::Binomial { pct } => {
                    quote! {
                        #field_name: rng.gen() > #pct
                    }
                }
            },
            HowToControlThisType::F32(rand_method_f32) => match rand_method_f32 {
                RandMethodF32::Uniform { start, end } => {
                    quote! { #field_name: rng.gen::<f32>() * (#end - #start) + #start }
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
                        quote! {
                            let width = rng.gen() * #width;
                            let height = rng.gen() * #height;

                            #field_name: vec2(#x, #y) - 0.5 * vec2(#width, #height) + vec2(width, height)

                        }
                    }
                    RandMethodVec2::Circle { x, y, radius } => {
                        quote! {{
                            let angle = rng.gen() * 2.0 * std::f32::consts::PI;
                            let dist = rng.gen(); // sqrt it to even out the sampling
                            #field_name: vec2(#x, #y) + vec2(angle.cos(), angle.sin()) * #radius * dist.sqrt()
                        }}
                    }
                }
            }
            HowToControlThisType::Vec(rand_method_vec) => match rand_method_vec {
                RandMethodVec::Length { min, max } => {
                    let ty = idents.data.ty;
                    let inside_type = nested_ident(&ty);
                    println!("INSIDE TYPE {:?}", inside_type);

                    let i = inside_type[2].clone();

                    quote! {{
                        let how_many = rng.gen_range(#min..#max) as usize;

                        let mut v = vec![];
                        for _ in 0..how_many {
                            v.push(#i::sample_dist(rng))
                        }

                        #field_name: v
                    }}
                }
            },
        };

        FieldTokensGen { for_make_gen }
    }

    fn from_override_struct(idents: StructIdents, func: &str) -> FieldTokensGen {
        let field_name = idents.data.ident.unwrap();

        let method: syn::Path = syn::parse_str(func).expect("Custom method is invalid path!");

        let for_make_gen = quote! { #field_name: #method() };

        FieldTokensGen { for_make_gen }
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
