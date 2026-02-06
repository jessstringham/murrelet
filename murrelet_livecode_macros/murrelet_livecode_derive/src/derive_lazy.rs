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
                quote! {murrelet_livecode::lazy::LazyVec2}
            }
            ControlType::F32_3 => {
                quote! {murrelet_livecode::lazy::LazyVec3}
            }
            ControlType::Color => {
                quote! {murrelet_livecode::lazy::LazyMurreletColor}
            }
            ControlType::LazyNodeF32 => {
                // already lazy...
                quote! { murrelet_livecode::lazy::LazyNodeF32 }
            }
            ControlType::AnglePi => {
                quote! { murrelet_livecode::lazy::LazyNodeF32 }
            }
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
                quote! { #ident.eval_lazy(ctx) }
            }
            ControlType::F32_3 => {
                quote! { #ident.eval_lazy(ctx) }
            }
            ControlType::Color => {
                quote! { #ident.eval_lazy(ctx) }
            }
            ControlType::Bool => quote! {#ident.eval_lazy(ctx)? > 0.0},
            ControlType::AnglePi => {
                // for number-like things, we also enable clamping! (it's a bit experimental though, be careful)
                quote! {murrelet_common::AnglePi::new(#ident.eval_lazy(ctx)?)}
            }
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

    fn for_world(&self, idents: StructIdents) -> TokenStream2 {
        let name = idents.name();
        let orig_ty = idents.orig_ty();
        match self.0 {
            ControlType::F32_2 => {
                quote! { #name: self.#name.eval_lazy(ctx)? }
            }
            ControlType::F32_3 => {
                quote! { #name: self.#name.eval_lazy(ctx)? }
            }
            ControlType::Color => {
                quote! { #name: self.#name.eval_lazy(ctx)? }
            }
            ControlType::Bool => quote! {#name: self.#name.eval_lazy(ctx)? > 0.0},
            ControlType::LazyNodeF32 => quote! {#name: self.#name.add_more_defs(ctx)? },
            ControlType::AnglePi => {
                quote! {#name: murrelet_common::AnglePi::new(self.#name.eval_lazy(ctx)?)}
            }
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

    fn for_world_option(&self, idents: StructIdents) -> TokenStream2 {
        let name = idents.name();
        let orig_ty = idents.orig_ty();
        match self.0 {
            ControlType::F32_2 => {
                quote! { #name: self.#name.map(|name| glam::vec2(name[0].eval_lazy(ctx)? as f32, name[1].eval_lazy(ctx)? as f32))}
            }
            ControlType::F32_3 => {
                quote! {#name: self.#name.map(|name| glam::vec3(name[0].eval_lazy(ctx)? as f32, name[1].eval_lazy(ctx)? as f32, name[2].eval_lazy(ctx)? as f32))}
            }
            ControlType::Color => {
                quote! {#name: self.#name.map(|name| murrelet_common::MurreletColor::hsva(name[0].eval_lazy(ctx)? as f32, name[1].eval_lazy(ctx)? as f32, name[2].eval_lazy(ctx)? as f32, name[3].eval_lazy(ctx)? as f32))}
            }
            ControlType::Bool => quote! {#name: self.#name.map(|name| name.eval_lazy(ctx)? > 0.0)},
            ControlType::AnglePi => {
                quote! {#name: self.#name.map(|name| murrelet_common::AnglePi::new(name.eval_lazy(ctx)?))}
            }
            ControlType::LazyNodeF32 => {
                quote! {#name: {
                        if let Some(name) = &self.#name {
                            let a = name.add_more_defs(ctx)?;
                            Some(a)
                        } else {
                            None
                        }
                    }
                }
            }
            _ => {
                // for number-like things, we also enable clamping! (it's a bit experimental though, be careful)
                let f32_out = match (idents.data.f32min, idents.data.f32max) {
                    (None, None) => quote! {
                        if let Some(name) = &self.#name {
                            let n = name.eval_lazy(ctx)?;
                            Some(n)
                        } else {
                            None
                        }
                    },
                    (None, Some(max)) => {
                        quote! {f32::min(self.#name.map(|name| name.eval_lazy(ctx)?, #max))}
                    }
                    (Some(min), None) => {
                        quote! {f32::max(#min, self.#name.map(|name| name.eval_lazy(ctx)?))}
                    }
                    (Some(min), Some(max)) => {
                        quote! {f32::min(f32::max(#min, self.#name.map(|name| name.eval_lazy(ctx)?), #max))}
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
                quote! { self.0.eval_lazy(ctx)? }
            }
            ControlType::F32_3 => {
                quote! { self.0.eval_lazy(ctx)? }
            }
            ControlType::Color => {
                quote! { self.0.eval_lazy(ctx)? }
            }
            ControlType::Bool => quote! {self.0.eval_lazy(ctx)? > 0.0},
            ControlType::AnglePi => {
                quote! {murrelet_common::AnglePi::new(self.0.eval_lazy(ctx)?)}
            }
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
    pub(crate) for_more_defs: TokenStream2,
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
        let for_more_defs = variants.iter().map(|x| x.for_more_defs.clone());

        quote! {
            #[derive(Debug, Clone, Default, murrelet_livecode_derive::LivecodeOnly)]
            #vis struct #lc_ident(#(#for_struct,)*);

            impl murrelet_livecode::lazy::IsLazy for #lc_ident {
                type Target = #name;
                fn eval_lazy(&self, ctx: &murrelet_livecode::expr::MixedEvalDefs) -> murrelet_livecode::types::LivecodeResult<#name> {
                    Ok(#name(#(#for_world,)*))
                }
                fn with_more_defs(&self, ctx: &murrelet_livecode::expr::MixedEvalDefs) -> murrelet_livecode::types::LivecodeResult<Self> {
                    Ok(Self(#(#for_more_defs,)*))
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
        let for_more_defs = variants.iter().map(|x| x.for_more_defs.clone());

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

                fn with_more_defs(&self, ctx: &murrelet_livecode::expr::MixedEvalDefs) -> murrelet_livecode::types::LivecodeResult<Self> {
                    Ok(Self {
                        #(#for_more_defs,)*
                    })
                }
            }
        }
    }

    fn make_enum_final(idents: ParsedFieldIdent, variants: Vec<FieldTokensLazy>) -> TokenStream2 {
        let new_enum_ident = idents.new_ident;
        let name = idents.name;
        let vis = idents.vis;
        let tags = idents.lazy_enum_tag;

        let for_struct = variants.iter().map(|x| x.for_struct.clone());
        let for_world = variants.iter().map(|x| x.for_world.clone());
        let for_more_defs = variants.iter().map(|x| x.for_more_defs.clone());

        quote! {
            #[derive(Debug, Clone, Default, murrelet_livecode_derive::LivecodeOnly)]
            #[allow(non_camel_case_types)]
            #tags
            #vis enum #new_enum_ident {
                #[default]
                DefaultNoop,
                #(#for_struct,)*
            }

            impl murrelet_livecode::lazy::IsLazy for #new_enum_ident {
                type Target = #name;
                fn eval_lazy(&self, ctx: &murrelet_livecode::expr::MixedEvalDefs) -> murrelet_livecode::types::LivecodeResult<#name> {
                    Ok(match self {
                        #new_enum_ident::DefaultNoop => panic!("fell back to default"), // can i just remove default?
                        #(#for_world,)*
                    })
                }

                fn with_more_defs(&self, ctx: &murrelet_livecode::expr::MixedEvalDefs) -> murrelet_livecode::types::LivecodeResult<Self> {
                    Ok(match self {
                        #new_enum_ident::DefaultNoop => #new_enum_ident::DefaultNoop,
                        #(#for_more_defs,)*
                    })
                }
            }
        }
    }

    fn from_newtype_struct_lazy(idents: StructIdents, _parent_ident: syn::Ident) -> Self {
        let orig_ty = idents.orig_ty();
        let parsed_type_info = ident_from_type(&orig_ty);
        let internal_type = parsed_type_info.main_type;

        let for_struct = {
            let new_inside_type = Self::new_ident(internal_type.clone());
            quote! {#new_inside_type}
        };

        let for_world = {
            quote! { self.0.clone() }
        };

        let for_more_defs = {
            quote! { self.0.with_more_defs(ctx)? }
        };

        FieldTokensLazy {
            for_struct,
            for_world,
            for_more_defs,
        }
    }

    fn from_newtype_struct_struct(
        idents: StructIdents,
        _parent_ident: syn::Ident,
    ) -> FieldTokensLazy {
        let orig_ty = idents.orig_ty();
        let parsed_type_info = ident_from_type(&orig_ty);
        let internal_type = parsed_type_info.main_type;

        let for_struct = {
            let new_inside_type = Self::new_ident(internal_type.clone());
            quote! {#new_inside_type}
        };

        let for_world = {
            quote! { self.0.eval_lazy(ctx)? }
        };

        let for_more_defs = {
            quote! { self.0.for_more_defs(ctx)? }
        };

        FieldTokensLazy {
            for_struct,
            for_world,
            for_more_defs,
        }
    }

    fn from_newtype_struct(idents: StructIdents, _parent_idents: syn::Ident) -> FieldTokensLazy {
        let ctrl = idents.control_type();

        let for_struct = {
            let t = LazyFieldType(ctrl).to_token();
            quote! {#t}
        };

        let for_world = LazyFieldType(ctrl).for_newtype_world(idents.clone());

        let for_more_defs = {
            quote! { self.0.with_more_defs(ctx)? }
        };

        FieldTokensLazy {
            for_struct,
            for_world,
            for_more_defs,
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

        let t = unnamed.first().unwrap().clone().ty;
        let parsed_data_type = ident_from_type(&t);

        let is_lazy = parsed_data_type.main_how_to.is_lazy();
        let for_struct = {
            let new_type = if is_lazy {
                parsed_data_type.main_type.clone()
            } else {
                update_to_lazy_ident(parsed_data_type.main_type)
            };

            quote! { #variant_ident(#new_type) }
        };

        // for world
        let for_world = if is_lazy {
            quote! { #new_enum_ident::#variant_ident(s) => #name::#variant_ident(s.clone()) }
        } else {
            quote! { #new_enum_ident::#variant_ident(s) => #name::#variant_ident(s.eval_lazy(ctx)?) }
        };

        let for_more_defs = if is_lazy {
            quote! { #new_enum_ident::#variant_ident(s) => #new_enum_ident::#variant_ident(s.clone()) }
        } else {
            quote! { #new_enum_ident::#variant_ident(s) => #new_enum_ident::#variant_ident(s.with_more_defs(ctx)?) }
        };

        FieldTokensLazy {
            for_struct,
            for_world,
            for_more_defs,
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

        let for_more_defs: TokenStream2 = {
            quote! { #new_enum_ident::#variant_ident => #new_enum_ident::#variant_ident }
        };

        FieldTokensLazy {
            for_struct,
            for_world,
            for_more_defs,
        }
    }

    fn from_noop_struct(idents: StructIdents) -> FieldTokensLazy {
        let name = idents.name();
        let new_ty = idents.orig_ty();
        let back_to_quote = idents.back_to_quote();

        let for_struct = {
            quote! {#back_to_quote #name: #new_ty}
        };
        let for_world: TokenStream2 = {
            quote! {#name: self.#name.clone()}
        };

        let for_more_defs: TokenStream2 = {
            quote! { #name: self.#name.clone() }
        };

        FieldTokensLazy {
            for_struct,
            for_world,
            for_more_defs,
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

        let _for_world = LazyFieldType(ctrl).for_world(idents.clone());

        let for_world = LazyFieldType(ctrl).for_world(idents.clone());
        let for_more_defs = {
            quote! { #name: self.#name.with_more_defs(ctx)? }
        };

        FieldTokensLazy {
            for_struct,
            for_world,
            for_more_defs,
        }
    }

    fn from_option(idents: StructIdents) -> Self {
        let name = idents.name();
        let back_to_quote = idents.back_to_quote();

        let s = ident_from_type(&idents.orig_ty());

        let ctrl = s.second_how_to.unwrap().get_control_type();

        let for_struct = {
            let t = LazyFieldType(ctrl).to_token();
            quote! {#back_to_quote #name: Option<#t>}
        };

        let for_world = LazyFieldType(ctrl).for_world_option(idents.clone());
        let for_more_defs = {
            quote! { #name: if let Some(value) = &self.#name { Some(value.with_more_defs(ctx)?) } else { None } }
        };

        FieldTokensLazy {
            for_struct,
            for_world,
            for_more_defs,
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
                e => panic!("lazy1 need vec something {:?}", e),
            };

            let new_ty = match wrapper {
                VecDepth::NotAVec => unreachable!("huh, parsing a not-vec in the vec function"), // why is it in this function?
                VecDepth::Vec => {
                    quote! {Vec<murrelet_livecode::types::LazyControlVecElement<murrelet_livecode::lazy::WrappedLazyType<#internal_type>>>}
                }
                VecDepth::VecVec => todo!(),
                VecDepth::VecControlVec => {
                    quote! { Vec<murrelet_livecode::types::LazyControlVecElement<murrelet_livecode::lazy::WrappedLazyType<Vec<#internal_type>>>> }
                }
            };
            quote! {#back_to_quote #name: #new_ty}
        };
        let for_world = {
            match how_to_control_internal {
                HowToControlThis::WithType(_, c) => {
                    // local variable...
                    let x_ident = syn::Ident::new("x", idents.name().span());
                    let c_expr = LazyFieldType(*c).for_world_func(
                        x_ident.clone(),
                        idents.data.f32min,
                        idents.data.f32max,
                    );
                    quote! {
                        #name: {
                            let expanded = murrelet_livecode::types::lazy_expand_vec_list(&self.#name, ctx)?;
                            expanded
                                .into_iter()
                                .map(|#x_ident| #c_expr)
                                .collect::<Result<Vec<_>, _>>()?
                        }
                    }
                }
                HowToControlThis::WithRecurse(_, RecursiveControlType::Struct) => {
                    quote! {
                        #name: {
                            let expanded = murrelet_livecode::types::lazy_expand_vec_list(&self.#name, ctx)?;
                            expanded
                                .into_iter()
                                .map(|x| x.eval_lazy(ctx))
                                .collect::<Result<Vec<_>, _>>()?
                        }
                    }
                }
                HowToControlThis::WithNone(_) => {
                    let target_type = parsed_type_info.internal_type();
                    let name = Self::new_ident(target_type.clone());
                    quote! {#name: self.#name.clone()}
                }
                e => panic!("lazy2 need vec something {:?}", e),
            }

            // match wrapper {
            //     VecDepth::NotAVec => unreachable!("huh, parsing a not-vec in the vec function"), // why is it in this function?
            //     VecDepth::Vec => {
            //         quote! {#name: self.#name.iter().map(|x| x.eval_lazy(ctx)).collect::<Result<Vec<_>, _>>()?}
            //     }
            //     VecDepth::VecVec => todo!("maybe support vec<vec<>>.."),
            // }
        };

        let for_more_defs = quote! {
            #name: self.#name
                .iter()
                .map(|item| item.with_more_defs(ctx))
                .collect::<murrelet_livecode::types::LivecodeResult<Vec<_>>>()?
        };

        FieldTokensLazy {
            for_struct,
            for_world,
            for_more_defs,
        }
    }

    fn from_newtype_recurse_struct_vec(idents: StructIdents) -> Self {
        let orig_ty = idents.orig_ty();

        let parsed_type_info = ident_from_type(&orig_ty);
        let how_to_control_internal = parsed_type_info.how_to_control_internal();

        let for_struct = {
            let new_ty = match how_to_control_internal {
                HowToControlThis::WithType(_, c) => LazyFieldType(*c).to_token(),
                HowToControlThis::WithRecurse(_, RecursiveControlType::Struct) => {
                    let internal_type = parsed_type_info.internal_type();
                    let name = Self::new_ident(internal_type);
                    quote! {#name}
                }
                HowToControlThis::WithNone(_) => {
                    let internal_type = parsed_type_info.internal_type();
                    let name = Self::new_ident(internal_type);
                    quote! {#name}
                }
                e => panic!("lazy3 need vec something {:?}", e),
            };

            quote! {Vec<#new_ty>}
        };
        let for_world = {
            quote! {self.0.iter().map(|x| x.eval_lazy(ctx)).collect::<Result<Vec<_>, _>>()?}
        };
        let for_more_defs = {
            quote! { self.0.iter().map(|x| x.with_more_defs(ctx)).collect::<murrelet_livecode::types::LivecodeResult<Vec<_>>>()? }
        };

        FieldTokensLazy {
            for_struct,
            for_world,
            for_more_defs,
        }
    }

    fn from_recurse_struct_unitcell(idents: StructIdents) -> FieldTokensLazy {
        let name = idents.name();
        let orig_ty = idents.orig_ty();
        let back_to_quote = idents.back_to_quote();

        let parsed_type_info = ident_from_type(&orig_ty);
        let how_to_control_internal = parsed_type_info.how_to_control_internal();

        let for_struct = {
            let new_ty = match how_to_control_internal {
                HowToControlThis::WithRecurse(_, RecursiveControlType::Struct) => {
                    let internal_type = parsed_type_info.internal_type();
                    let name = update_to_lazy_ident(internal_type);
                    quote! {murrelet_livecode::unitcells::UnitCells<#name>}
                }

                HowToControlThis::WithRecurse(_, RecursiveControlType::StructLazy) => {
                    let internal_type = parsed_type_info.internal_type();
                    let name = internal_type;
                    quote! {murrelet_livecode::unitcells::UnitCells<#name>}
                }

                e => panic!("need lazy something {:?}", e),
            };

            quote! {#back_to_quote #name: #new_ty}
        };

        let for_world = {
            if how_to_control_internal.is_lazy() {
                quote! {#name: self.#name.clone() }
            } else {
                quote! {#name: {
                    let c = self.#name.iter().map(|x|
                        x.node.eval_lazy(ctx).map(|r| x.to_other_type(r))
                    ).collect::<Result<Vec<_>, _>>()?;
                    murrelet_livecode::unitcells::UnitCells::new(c)
                }
                }
            }
        };

        let for_more_defs = {
            quote! { #name: self.#name.clone() }
        };

        FieldTokensLazy {
            for_struct,
            for_world,
            for_more_defs,
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

        let for_more_defs = {
            quote! { #name: self.#name.with_more_defs(ctx)? }
        };

        FieldTokensLazy {
            for_struct,
            for_world,
            for_more_defs,
        }
    }

    fn new_ident(name: syn::Ident) -> syn::Ident {
        update_to_lazy_ident(name)
    }

    fn from_recurse_struct_lazy(idents: StructIdents) -> Self {
        Self::from_noop_struct(idents)
    }
}
