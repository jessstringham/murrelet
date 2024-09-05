use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::parser::*;

pub(crate) fn update_to_lazy_ident(name: syn::Ident) -> syn::Ident {
    prefix_ident("Lazy", name)
}

pub struct LazyFieldType(ControlType);

impl LazyFieldType {
    fn to_token(&self) -> TokenStream2 {
        match self.0 {
            ControlType::Bool => quote! {murrelet_livecode::lazy::LazyNodeF32}, // we'll just check if it's above 0
            ControlType::F32 => quote! {murrelet_livecode::lazy::LazyNodeF32},
            ControlType::F32_2 => {
                quote! {Vec<murrelet_livecode::lazy::LazyNodeF32>}
            }
            ControlType::F32_3 => {
                quote! {Vec<murrelet_livecode::lazy::LazyNodeF32>}
            }
            ControlType::Color => {
                quote! {Vec<murrelet_livecode::lazy::LazyNodeF32>}
            }
            ControlType::LazyNodeF32 => {
                // already lazy...
                quote! { murrelet_livecode::lazy::LazyNodeF32 }
            }
            // ControlType::LinSrgbaUnclamped => quote!{[murrelet_livecode::livecode::ControlF32; 4]},
            _ => panic!("unitcell doesn't have this one yet"),
        }
    }

    fn for_world_func(
        &self,
        ident: syn::Ident,
        f32min: Option<f32>,
        f32max: Option<f32>,
    ) -> TokenStream2 {
        match self.0 {
            ControlType::F32_2 => {
                quote! { murrelet_livecode::lazy::eval_lazy_vec2(#ident) }
            }
            ControlType::F32_3 => {
                quote! { murrelet_livecode::lazy::eval_lazy_vec3(#ident) }
            }
            ControlType::Color => {
                quote! { murrelet_livecode::lazy::eval_lazy_color(#ident) }
            }
            ControlType::Bool => quote! {#ident.eval_lazy(ctx)? > 0.0},
            _ => {
                // for number-like things, we also enable clamping! (it's a bit experimental though, be careful)
                let f32_out = match (f32min, f32max) {
                    (None, None) => quote! {#ident.eval_lazy(ctx)},
                    (None, Some(max)) => quote! {Ok(f32::min(#ident.eval_lazy(ctx)?, #max))},
                    (Some(min), None) => quote! {Ok(f32::max(#min, #ident.eval_lazy(ctx)?))},
                    (Some(min), Some(max)) => {
                        quote! {Ok(f32::min(f32::max(#min, #ident.eval_lazy(ctx)?), #max))}
                    }
                };
                quote! {#f32_out}
            }
        }
    }

    // todo reuse for world func
    fn for_world(&self, idents: StructIdents) -> TokenStream2 {
        let name = idents.name();
        let orig_ty = idents.orig_ty();
        match self.0 {
            ControlType::F32_2 => {
                // quote! {#name: self.#name.eval_lazy(ctx)?}
                quote! { #name: glam::vec2(self.#name[0].eval_lazy(ctx)? as f32, self.#name[1].eval_lazy(ctx)? as f32)}
            }
            ControlType::F32_3 => {
                quote! {#name: glam::vec3(self.#name[0].eval_lazy(ctx)? as f32, self.#name[1].eval_lazy(ctx)? as f32, self.#name[2].eval_lazy(ctx)? as f32)}
                // quote! {#name: self.#name.eval_lazy(ctx)?}
            }
            ControlType::Color => {
                quote! {#name: murrelet_common::MurreletColor::hsva(self.#name[0].eval_lazy(ctx)? as f32, self.#name[1].eval_lazy(ctx)? as f32, self.#name[2].eval_lazy(ctx)? as f32, self.#name[3].eval_lazy(ctx)? as f32)}
                // quote! {#name: self.#name.eval_lazy(ctx)?}
            }
            // ControlType::LinSrgbaUnclamped => quote!{#name: murrelet_livecode::livecode::ControlF32::hsva_unclamped(&self.#name, w)},
            ControlType::Bool => quote! {#name: self.#name.eval_lazy(ctx)? > 0.0},
            // _ => quote!{#name: self.#name.eval_lazy(ctx)? as #orig_ty}
            _ => {
                // for number-like things, we also enable clamping! (it's a bit experimental though, be careful)
                let f32_out = match (idents.data.f32min, idents.data.f32max) {
                    (None, None) => quote! {self.#name.eval_lazy(ctx)?},
                    (None, Some(max)) => quote! {f32::min(self.#name.eval_lazy(ctx)?, #max)},
                    (Some(min), None) => quote! {f32::max(#min, self.#name.eval_lazy(ctx)?)},
                    (Some(min), Some(max)) => {
                        quote! {f32::min(f32::max(#min, self.#name.eval_lazy(ctx)?), #max)}
                    }
                };
                quote! {#name: #f32_out as #orig_ty}
            }
        }
    }

    fn for_newtype_world(&self, idents: StructIdents) -> TokenStream2 {
        let orig_ty = idents.orig_ty();
        match self.0 {
            ControlType::F32_2 => {
                quote! {vec2(self.0[0].eval_lazy(ctx)? as f32, self.0[1].eval_lazy(ctx)? as f32)}
            }
            // ControlType::F32_3 => quote!{murrelet_livecode::livecode::ControlF32::vec3(&self.0, w)},
            ControlType::Color => {
                quote! {MurreletColor::hsva(self.0[0].eval_lazy(ctx)? as f32, self.0[1].eval_lazy(ctx)? as f32, self.0[2].eval_lazy(ctx)? as f32, self.0[3].eval_lazy(ctx)? as f32)}
            }
            // ControlType::LinSrgbaUnclamped => quote!{murrelet_livecode::livecode::ControlF32::hsva_unclamped(&self.0, w)},
            ControlType::Bool => quote! {self.0.eval_lazy(ctx)? > 0.0},
            // _ => quote!{self.0.eval_lazy(ctx)? as #orig_ty}
            _ => {
                let f32_out = match (idents.data.f32min, idents.data.f32max) {
                    (None, None) => quote! {self.0.eval_lazy(ctx)?},
                    (None, Some(max)) => quote! {f32::min(self.0.eval_lazy(ctx)?, #max)},
                    (Some(min), None) => quote! {f32::max(#min, self.0.eval_lazy(ctx)?)},
                    (Some(min), Some(max)) => {
                        quote! {f32::min(f32::max(#min, self.0.eval_lazy(ctx)?), #max)}
                    }
                };
                quote! {#f32_out as #orig_ty}
            }
        }
    }
}

pub(crate) struct FieldTokensLazy {
    pub(crate) for_struct: TokenStream2,
    pub(crate) for_world: TokenStream2,
}
impl GenFinal for FieldTokensLazy {
    fn make_newtype_struct_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensLazy>,
    ) -> TokenStream2 {
        let lc_ident = idents.new_ident;
        let name = idents.name;
        let vis = idents.vis;

        let for_struct = variants.iter().map(|x| x.for_struct.clone());
        let for_world = variants.iter().map(|x| x.for_world.clone());

        quote! {
            #[derive(Debug, Clone, Default, murrelet_livecode_derive::LivecodeOnly)]
            #vis struct #lc_ident(#(#for_struct,)*);

            impl murrelet_livecode::lazy::IsLazy for #lc_ident {
                type Target = #name;
                fn eval_lazy(&self, ctx: &murrelet_livecode::expr::MixedEvalDefs) -> murrelet_livecode::types::LivecodeResult<#name> {
                    Ok(#name(#(#for_world,)*))
                }
            }

        }
    }

    fn make_struct_final(idents: ParsedFieldIdent, variants: Vec<FieldTokensLazy>) -> TokenStream2 {
        let lc_ident = idents.new_ident;
        let name = idents.name;
        let vis = idents.vis;

        let for_struct = variants.iter().map(|x| x.for_struct.clone());
        let for_world = variants.iter().map(|x| x.for_world.clone());

        quote! {
            #[derive(Debug, Clone, Default, murrelet_livecode_derive::LivecodeOnly)]
            #vis struct #lc_ident {
                #(#for_struct,)*
            }

            impl murrelet_livecode::lazy::IsLazy for #lc_ident {
                type Target = #name;
                fn eval_lazy(&self, ctx: &murrelet_livecode::expr::MixedEvalDefs) -> murrelet_livecode::types::LivecodeResult<#name> {
                    Ok(#name {
                        #(#for_world,)*
                    })
                }
            }
        }
    }

    fn make_enum_final(idents: ParsedFieldIdent, variants: Vec<FieldTokensLazy>) -> TokenStream2 {
        let new_enum_ident = idents.new_ident;
        let name = idents.name;
        let vis = idents.vis;

        let for_struct = variants.iter().map(|x| x.for_struct.clone());
        let for_world = variants.iter().map(|x| x.for_world.clone());

        quote! {
            #[derive(Debug, Clone, Default, murrelet_livecode_derive::LivecodeOnly)]
            #[allow(non_camel_case_types)]
            #vis enum #new_enum_ident {
                #[default]
                Noop,
                #(#for_struct,)*
            }

            impl murrelet_livecode::lazy::IsLazy for #new_enum_ident {
                type Target = #name;
                fn eval_lazy(&self, ctx: &murrelet_livecode::expr::MixedEvalDefs) -> murrelet_livecode::types::LivecodeResult<#name> {
                    Ok(match self {
                        #new_enum_ident::Noop => panic!("fell back to default"), // can i just remove default?
                        #(#for_world,)*
                    })
                }
            }
        }
    }

    fn from_newtype_struct(idents: StructIdents, _parent_idents: syn::Ident) -> FieldTokensLazy {
        let ctrl = idents.control_type();

        let for_struct = {
            let t = LazyFieldType(ctrl).to_token();
            quote! {#t}
        };

        let for_world = LazyFieldType(ctrl).for_newtype_world(idents.clone());

        FieldTokensLazy {
            for_struct,
            for_world,
        }
    }

    // enum
    // Arc(CurveArc)
    fn from_unnamed_enum(idents: EnumIdents) -> FieldTokensLazy {
        let variant_ident = idents.variant_ident();
        let name = idents.enum_ident();
        let new_enum_ident = Self::new_ident(name.clone());

        let unnamed = idents.data.fields.fields;

        // for struct
        if unnamed.len() != 1 {
            panic!("multiple fields not supported")
        };

        let for_struct = {
            let t = unnamed.first().unwrap().clone().ty;
            let DataFromType { main_type, .. } = ident_from_type(&t);
            let new_type = update_to_lazy_ident(main_type);
            quote! { #variant_ident(#new_type) }
        };

        // for world
        let for_world = quote! { #new_enum_ident::#variant_ident(s) => #name::#variant_ident(s.eval_lazy(ctx)?) };

        FieldTokensLazy {
            for_struct,
            for_world,
        }
    }

    fn from_unit_enum(idents: EnumIdents) -> FieldTokensLazy {
        let variant_ident = idents.variant_ident();
        let name = idents.enum_ident();
        let new_enum_ident = Self::new_ident(name.clone());

        let for_struct = {
            quote! { #variant_ident }
        };
        let for_world: TokenStream2 = {
            quote! { #new_enum_ident::#variant_ident => #name::#variant_ident }
        };

        FieldTokensLazy {
            for_struct,
            for_world,
        }
    }

    fn from_noop_struct(idents: StructIdents) -> FieldTokensLazy {
        let name = idents.name();
        let new_ty = idents.orig_ty();

        let for_struct = {
            quote! {#name: #new_ty}
        };
        let for_world: TokenStream2 = {
            quote! {#name: self.#name.clone()}
        };

        FieldTokensLazy {
            for_struct,
            for_world,
        }
    }

    fn from_type_struct(idents: StructIdents) -> FieldTokensLazy {
        let name = idents.name();
        let back_to_quote = idents.back_to_quote();

        let ctrl = idents.control_type();

        let for_struct = {
            let t = LazyFieldType(ctrl).to_token();
            quote! {#back_to_quote #name: #t}
        };

        let for_world = LazyFieldType(ctrl).for_world(idents.clone());

        FieldTokensLazy {
            for_struct,
            for_world,
        }
    }

    // Vec<CurveSegment>, Vec<f32>
    fn from_recurse_struct_vec(idents: StructIdents) -> FieldTokensLazy {
        let name = idents.name();
        let orig_ty = idents.orig_ty();
        let back_to_quote = idents.back_to_quote();

        let parsed_type_info = ident_from_type(&orig_ty);
        let how_to_control_internal = parsed_type_info.how_to_control_internal();
        let wrapper = parsed_type_info.wrapper_type();

        println!("how_to_control_internal {:?}", how_to_control_internal);

        let for_struct = {
            let internal_type = match how_to_control_internal {
                HowToControlThis::WithType(_, c) => LazyFieldType(*c).to_token(),
                HowToControlThis::WithRecurse(_, RecursiveControlType::Struct) => {
                    let target_type = parsed_type_info.internal_type();
                    let name = Self::new_ident(target_type.clone());
                    quote! {#name}
                }
                HowToControlThis::WithNone(_) => {
                    let target_type = parsed_type_info.internal_type();
                    let name = Self::new_ident(target_type.clone());
                    quote! {#name}
                }
                e => panic!("need vec something {:?}", e),
            };

            let new_ty = match wrapper {
                VecDepth::NotAVec => unreachable!("huh, parsing a not-vec in the vec function"), // why is it in this function?
                VecDepth::Vec => quote! {Vec<#internal_type>},
                VecDepth::VecVec => todo!(),
            };
            quote! {#back_to_quote #name: #new_ty}
        };
        let for_world = {
            match how_to_control_internal {
                HowToControlThis::WithType(_, c) => {
                    // local variable...
                    let x = syn::Ident::new("x", idents.name().span());
                    let c =
                        LazyFieldType(*c).for_world_func(x, idents.data.f32min, idents.data.f32max);
                    quote! {#name: self.#name.iter().map(|x| #c).collect::<Result<Vec<_>, _>>()?}
                }
                HowToControlThis::WithRecurse(_, RecursiveControlType::Struct) => {
                    quote! {#name: self.#name.iter().map(|x| x.eval_lazy(ctx)).collect::<Result<Vec<_>, _>>()?}
                }
                HowToControlThis::WithNone(_) => {
                    let target_type = parsed_type_info.internal_type();
                    let name = Self::new_ident(target_type.clone());
                    quote! {#name: self.#name.clone()}
                }
                e => panic!("need vec something {:?}", e),
            }

            // match wrapper {
            //     VecDepth::NotAVec => unreachable!("huh, parsing a not-vec in the vec function"), // why is it in this function?
            //     VecDepth::Vec => {
            //         quote! {#name: self.#name.iter().map(|x| x.eval_lazy(ctx)).collect::<Result<Vec<_>, _>>()?}
            //     }
            //     VecDepth::VecVec => todo!("maybe support vec<vec<>>.."),
            // }
        };

        FieldTokensLazy {
            for_struct,
            for_world,
        }
    }

    fn from_newtype_recurse_struct_vec(idents: StructIdents) -> Self {
        let orig_ty = idents.orig_ty();

        let for_struct = {
            let ref_lc_ident = if let DataFromType {
                second_type: Some(second_ty_ident),
                ..
            } = ident_from_type(&orig_ty)
            {
                let infer =
                    HowToControlThis::from_type_str(second_ty_ident.clone().to_string().as_ref());

                match infer {
                    HowToControlThis::WithType(_, c) => LazyFieldType(c).to_token(),
                    HowToControlThis::WithRecurse(_, RecursiveControlType::Struct) => {
                        let name = Self::new_ident(second_ty_ident.clone());
                        quote! {#name}
                    }
                    HowToControlThis::WithNone(_) => {
                        let name = Self::new_ident(second_ty_ident.clone());
                        quote! {#name}
                    }
                    e => panic!("need vec something {:?}", e),
                }
            } else {
                panic!("vec missing second type");
            };
            quote! {Vec<#ref_lc_ident>}
        };
        let for_world = {
            quote! {self.0.iter().map(|x| x.eval_lazy(ctx)).collect::<Result<Vec<_>, _>>()?}
        };

        FieldTokensLazy {
            for_struct,
            for_world,
        }
    }

    fn from_recurse_struct_unitcell(idents: StructIdents) -> FieldTokensLazy {
        let name = idents.name();
        let orig_ty = idents.orig_ty();
        let back_to_quote = idents.back_to_quote();

        let (for_struct, _new_ty): (TokenStream2, TokenStream2) = {
            let new_ty = {
                let ref_lc_ident = if let DataFromType {
                    second_type: Some(second_ty_ident),
                    ..
                } = ident_from_type(&orig_ty)
                {
                    let infer = HowToControlThis::from_type_str(
                        second_ty_ident.clone().to_string().as_ref(),
                    );

                    match infer {
                        HowToControlThis::WithRecurse(_, RecursiveControlType::Struct) => {
                            let name = update_to_lazy_ident(second_ty_ident.clone());
                            quote! {murrelet_livecode::unitcells::UnitCells<#name>}
                        }

                        e => panic!("need lazy something {:?}", e),
                    }
                } else {
                    panic!("unitcell missing second type")
                };

                quote! {#ref_lc_ident}
            };

            (quote! {#back_to_quote #name: #new_ty}, new_ty.clone())
        };

        let for_world = {
            quote! {#name: {
                let c = self.#name.iter().map(|x|
                        x.node.eval_lazy(ctx).map(|r| x.to_other_type(r))
                    ).collect::<Result<Vec<_>, _>>()?;
                    murrelet_livecode::unitcells::UnitCells::new(c)
                }
            }
        };

        FieldTokensLazy {
            for_struct,
            for_world,
        }
    }

    fn from_recurse_struct_struct(idents: StructIdents) -> Self {
        let name = idents.name();
        let orig_ty = idents.orig_ty();
        let back_to_quote = idents.back_to_quote();

        let for_struct = {
            let new_ty = {
                let DataFromType { main_type, .. } = ident_from_type(&orig_ty);
                let ref_lc_ident = Self::new_ident(main_type.clone());

                quote! {#ref_lc_ident}
            };

            quote! {#back_to_quote #name: #new_ty}
        };
        let for_world = {
            quote! {#name: self.#name.eval_lazy(ctx)?}
        };

        FieldTokensLazy {
            for_struct,
            for_world,
        }
    }

    fn new_ident(name: syn::Ident) -> syn::Ident {
        update_to_lazy_ident(name)
    }
}
