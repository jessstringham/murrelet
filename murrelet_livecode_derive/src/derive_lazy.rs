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

    // orig_ty is the Vec element type; the numeric arm narrows each f32 to it
    // (e.g. `Vec<usize>`), mirroring the scalar `as #orig_ty` on for_world_target.
    fn for_world_func(
        &self,
        ident: syn::Ident,
        orig_ty: syn::Ident,
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
                match (f32min, f32max) {
                    (None, None) => quote! {#ident.eval_lazy(ctx).map(|v| v as #orig_ty)},
                    (None, Some(max)) => {
                        quote! {Ok(f32::min(#ident.eval_lazy(ctx)?, #max) as #orig_ty)}
                    }
                    (Some(min), None) => {
                        quote! {Ok(f32::max(#min, #ident.eval_lazy(ctx)?) as #orig_ty)}
                    }
                    (Some(min), Some(max)) => {
                        quote! {Ok(f32::min(f32::max(#min, #ident.eval_lazy(ctx)?), #max) as #orig_ty)}
                    }
                }
            }
        }
    }

    fn for_world(&self, idents: StructIdents) -> TokenStream2 {
        let name = idents.name();
        let rest = self.for_world_target(
            quote!(self.#name),
            idents.orig_ty(),
            idents.data.f32min,
            idents.data.f32max,
        );
        quote! { #name: #rest }
    }

    // V(prim) for_world. target is the place to read (`self.foo` / `s`).
    fn for_world_target(
        &self,
        target: TokenStream2,
        orig_ty: syn::Type,
        f32min: Option<f32>,
        f32max: Option<f32>,
    ) -> TokenStream2 {
        match self.0 {
            ControlType::F32_2 => quote! { #target.eval_lazy(ctx)? },
            ControlType::F32_3 => quote! { #target.eval_lazy(ctx)? },
            ControlType::Color => quote! { #target.eval_lazy(ctx)? },
            ControlType::Bool => quote! { #target.eval_lazy(ctx)? > 0.0 },
            ControlType::LazyNodeF32 => quote! { #target.add_more_defs(ctx)? },
            ControlType::AnglePi => {
                quote! { murrelet_common::AnglePi::new(#target.eval_lazy(ctx)?) }
            }
            _ => {
                let f32_out = match (f32min, f32max) {
                    (None, None) => quote! { #target.eval_lazy(ctx)? },
                    (None, Some(max)) => quote! { f32::min(#target.eval_lazy(ctx)?, #max) },
                    (Some(min), None) => quote! { f32::max(#min, #target.eval_lazy(ctx)?) },
                    (Some(min), Some(max)) => {
                        quote! { f32::min(f32::max(#min, #target.eval_lazy(ctx)?), #max) }
                    }
                };
                quote! { #f32_out as #orig_ty }
            }
        }
    }

    fn for_world_option(&self, idents: StructIdents) -> TokenStream2 {
        let name = idents.name();
        let orig_ty = idents.orig_ty();
        match self.0 {
            ControlType::F32_2 | ControlType::F32_3 | ControlType::Color => {
                quote! {#name: {
                    if let Some(name) = &self.#name {
                        Some(name.eval_lazy(ctx)?)
                    } else {
                        None
                    }
                }}
            }
            ControlType::Bool => quote! {#name: {
                if let Some(name) = &self.#name {
                    Some(name.eval_lazy(ctx)? > 0.0)
                } else {
                    None
                }
            }},
            ControlType::AnglePi => {
                quote! {#name: {
                    if let Some(name) = &self.#name {
                        Some(murrelet_common::AnglePi::new(name.eval_lazy(ctx)?))
                    } else {
                        None
                    }
                }}
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
                let inner_ty = ident_from_type(&orig_ty).inside_type().to_quote();
                let clamped = match (idents.data.f32min, idents.data.f32max) {
                    (None, None) => quote! { v },
                    (None, Some(max)) => quote! { f32::min(v, #max) },
                    (Some(min), None) => quote! { f32::max(#min, v) },
                    (Some(min), Some(max)) => quote! { f32::min(f32::max(#min, v), #max) },
                };
                quote! {#name: {
                    if let Some(name) = &self.#name {
                        let v = name.eval_lazy(ctx)?;
                        Some(#clamped as #inner_ty)
                    } else {
                        None
                    }
                }}
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

// Shared arm-builders, lazy side. target_ref: passed to lazy_expand_vec_list
// (`&self.foo` / `s` / `&self.0`). target_iter: receives `.iter()`
// (`self.foo` / `s` / `self.0`). inner_eval: closure body for each expanded
// element (`x.eval_lazy(ctx)` for Struct payloads, the LazyFieldType
// for_world_func output for primitives).
fn lazy_vec_for_world_expr(
    target_ref: TokenStream2,
    inner_eval: TokenStream2,
) -> TokenStream2 {
    quote! {
        {
            let expanded = murrelet_livecode::types::lazy_expand_vec_list(#target_ref, ctx)?;
            expanded
                .into_iter()
                .map(|x| #inner_eval)
                .collect::<Result<Vec<_>, _>>()?
        }
    }
}

fn lazy_vec_for_more_defs_expr(target_iter: TokenStream2) -> TokenStream2 {
    quote! {
        #target_iter.iter()
            .map(|item| item.with_more_defs(ctx))
            .collect::<murrelet_livecode::types::LivecodeResult<Vec<_>>>()?
    }
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
            #[derive(Debug, Clone, Default, murrelet_livecode::LivecodeOnly)]
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
            #[derive(Debug, Clone, Default, murrelet_livecode::LivecodeOnly)]
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
            #[derive(Debug, Clone, Default, murrelet_livecode::LivecodeOnly)]
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
        let internal_type = parsed_type_info.main_type();

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
        let internal_type = parsed_type_info.main_type();

        let for_struct = {
            let new_inside_type = Self::new_ident(internal_type.clone());
            quote! {#new_inside_type}
        };

        let for_world = {
            quote! { self.0.eval_lazy(ctx)? }
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
        match idents.single_inner_how_to() {
            HowToControlThis::WithType(_) => Self::from_unnamed_enum_primitive(idents),
            HowToControlThis::WithRecurse(RecursiveControlType::Vec) => {
                Self::from_unnamed_enum_vec(idents)
            }
            HowToControlThis::WithRecurse(RecursiveControlType::Option) => {
                Self::from_unnamed_enum_option(idents)
            }
            HowToControlThis::WithRecurse(RecursiveControlType::Struct) => {
                Self::from_unnamed_enum_struct(idents)
            }
            HowToControlThis::WithRecurse(RecursiveControlType::StructLazy) => {
                Self::from_unnamed_enum_struct_lazy(idents)
            }
            e => panic!("(lazy, enum variant) not supported: {:?}", e),
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

        let ctrl = s.second_how_to().unwrap().get_control_type();

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
                HowToControlThis::WithType(c) => LazyFieldType(*c).to_token(),
                HowToControlThis::WithRecurse(RecursiveControlType::Struct) => {
                    let target_type = parsed_type_info.internal_type();
                    let name = Self::new_ident(target_type.clone());
                    quote! {#name}
                }
                HowToControlThis::WithNone => {
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
                HowToControlThis::WithType(c) => {
                    let x_ident = syn::Ident::new("x", idents.name().span());
                    let c_expr = LazyFieldType(*c).for_world_func(
                        x_ident.clone(),
                        parsed_type_info.internal_type(),
                        idents.data.f32min,
                        idents.data.f32max,
                    );
                    let expr = lazy_vec_for_world_expr(quote!(&self.#name), c_expr);
                    quote! { #name: #expr }
                }
                HowToControlThis::WithRecurse(RecursiveControlType::Struct) => {
                    let expr = lazy_vec_for_world_expr(quote!(&self.#name), quote!(x.eval_lazy(ctx)));
                    quote! { #name: #expr }
                }
                HowToControlThis::WithNone => {
                    let target_type = parsed_type_info.internal_type();
                    let name = Self::new_ident(target_type.clone());
                    quote! {#name: self.#name.clone()}
                }
                e => panic!("lazy2 need vec something {:?}", e),
            }
        };

        let for_more_defs = {
            let expr = lazy_vec_for_more_defs_expr(quote!(self.#name));
            quote! { #name: #expr }
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
                HowToControlThis::WithType(c) => LazyFieldType(*c).to_token(),
                HowToControlThis::WithRecurse(RecursiveControlType::Struct) => {
                    let internal_type = parsed_type_info.internal_type();
                    let name = Self::new_ident(internal_type);
                    quote! {#name}
                }
                HowToControlThis::WithRecurse(RecursiveControlType::StructLazy) => {
                    let internal_type = parsed_type_info.internal_type();
                    quote! {#internal_type}
                }
                HowToControlThis::WithNone => {
                    let internal_type = parsed_type_info.internal_type();
                    let name = Self::new_ident(internal_type);
                    quote! {#name}
                }
                e => panic!("lazy3 need vec something {:?}", e),
            };

            quote! {Vec<murrelet_livecode::types::LazyControlVecElement<murrelet_livecode::lazy::WrappedLazyType<#new_ty>>>}
        };
        let for_world = match how_to_control_internal {
            HowToControlThis::WithType(c) => {
                let x_ident = syn::Ident::new("x", proc_macro2::Span::call_site());
                let c_expr = LazyFieldType(*c).for_world_func(
                    x_ident.clone(),
                    parsed_type_info.internal_type(),
                    idents.data.f32min,
                    idents.data.f32max,
                );
                lazy_vec_for_world_expr(quote!(&self.0), c_expr)
            }
            HowToControlThis::WithRecurse(RecursiveControlType::Struct) => {
                lazy_vec_for_world_expr(quote!(&self.0), quote!(x.eval_lazy(ctx)))
            }
            HowToControlThis::WithNone => quote! {self.0.clone()},
            e => panic!("lazy3 for_world need vec something {:?}", e),
        };
        let for_more_defs = quote! {
            self.0.iter().map(|x| x.with_more_defs(ctx)).collect::<murrelet_livecode::types::LivecodeResult<Vec<_>>>()?
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
                HowToControlThis::WithRecurse(RecursiveControlType::Struct) => {
                    let internal_type = parsed_type_info.internal_type();
                    let name = update_to_lazy_ident(internal_type);
                    quote! {murrelet_livecode::unitcells::UnitCells<#name>}
                }

                HowToControlThis::WithRecurse(RecursiveControlType::StructLazy) => {
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
                let main_type = ident_from_type(&orig_ty).main_type();
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

impl FieldTokensLazy {
    // V(MyStruct) -> V(LazyMyStruct)
    fn from_unnamed_enum_struct(idents: EnumIdents) -> FieldTokensLazy {
        let variant_ident = idents.variant_ident();
        let name = idents.enum_ident();
        let new_enum_ident = Self::new_ident(name.clone());

        let main_type = ident_from_type(&idents.single_inner_ty()).main_type();
        let new_type = update_to_lazy_ident(main_type);

        let for_struct = quote! { #variant_ident(#new_type) };
        let for_world = quote! {
            #new_enum_ident::#variant_ident(s) => #name::#variant_ident(s.eval_lazy(ctx)?)
        };
        let for_more_defs = quote! {
            #new_enum_ident::#variant_ident(s) => #new_enum_ident::#variant_ident(s.with_more_defs(ctx)?)
        };

        FieldTokensLazy {
            for_struct,
            for_world,
            for_more_defs,
        }
    }

    // V(LazyMyStruct) -> V(LazyMyStruct) — already lazy, kept as-is
    fn from_unnamed_enum_struct_lazy(idents: EnumIdents) -> FieldTokensLazy {
        let variant_ident = idents.variant_ident();
        let name = idents.enum_ident();
        let new_enum_ident = Self::new_ident(name.clone());

        let new_type = ident_from_type(&idents.single_inner_ty()).main_type();

        let for_struct = quote! { #variant_ident(#new_type) };
        let for_world = quote! {
            #new_enum_ident::#variant_ident(s) => #name::#variant_ident(s.clone())
        };
        let for_more_defs = quote! {
            #new_enum_ident::#variant_ident(s) => #new_enum_ident::#variant_ident(s.clone())
        };

        FieldTokensLazy {
            for_struct,
            for_world,
            for_more_defs,
        }
    }

    // V(f32) -> V(LazyNodeF32) ; V(Vec2) -> V(LazyVec2) ; etc.
    fn from_unnamed_enum_primitive(idents: EnumIdents) -> FieldTokensLazy {
        let variant_ident = idents.variant_ident();
        let name = idents.enum_ident();
        let new_enum_ident = Self::new_ident(name.clone());

        let orig_ty = idents.single_inner_ty();
        let ctrl = idents.single_inner_how_to().get_control_type();
        let new_type = LazyFieldType(ctrl).to_token();

        // No field metadata on enum variants -> no f32min/f32max clamping.
        let inner_world_expr =
            LazyFieldType(ctrl).for_world_target(quote!(s), orig_ty, None, None);

        let for_struct = quote! { #variant_ident(#new_type) };
        let for_world = quote! {
            #new_enum_ident::#variant_ident(s) => #name::#variant_ident(#inner_world_expr)
        };
        let for_more_defs = quote! {
            #new_enum_ident::#variant_ident(s) => #new_enum_ident::#variant_ident(s.with_more_defs(ctx)?)
        };

        FieldTokensLazy {
            for_struct,
            for_world,
            for_more_defs,
        }
    }

    // V(Vec<T>) -> V(Vec<LazyControlVecElement<WrappedLazyType<LazyT>>>)
    fn from_unnamed_enum_vec(idents: EnumIdents) -> FieldTokensLazy {
        let variant_ident = idents.variant_ident();
        let name = idents.enum_ident();
        let new_enum_ident = Self::new_ident(name.clone());

        let parsed = ident_from_type(&idents.single_inner_ty());
        let inner_how_to = parsed.second_how_to().expect("Vec needs an inner type");
        let inner_type = match inner_how_to {
            HowToControlThis::WithType(c) => LazyFieldType(c).to_token(),
            HowToControlThis::WithRecurse(RecursiveControlType::Struct) => {
                let t = parsed.internal_type();
                let n = update_to_lazy_ident(t);
                quote! { #n }
            }
            HowToControlThis::WithRecurse(RecursiveControlType::StructLazy) => {
                let t = parsed.internal_type();
                quote! { #t }
            }
            e => panic!("(lazy, enum variant Vec) inner not supported: {:?}", e),
        };

        let world_expr = lazy_vec_for_world_expr(quote!(s), quote!(x.eval_lazy(ctx)));
        let more_defs_expr = lazy_vec_for_more_defs_expr(quote!(s));

        let for_struct = quote! {
            #variant_ident(Vec<murrelet_livecode::types::LazyControlVecElement<murrelet_livecode::lazy::WrappedLazyType<#inner_type>>>)
        };
        let for_world = quote! {
            #new_enum_ident::#variant_ident(s) => #name::#variant_ident(#world_expr)
        };
        let for_more_defs = quote! {
            #new_enum_ident::#variant_ident(s) => #new_enum_ident::#variant_ident(#more_defs_expr)
        };

        FieldTokensLazy {
            for_struct,
            for_world,
            for_more_defs,
        }
    }

    // V(Option<T>) -> V(Option<LazyT>)
    fn from_unnamed_enum_option(idents: EnumIdents) -> FieldTokensLazy {
        let variant_ident = idents.variant_ident();
        let name = idents.enum_ident();
        let new_enum_ident = Self::new_ident(name.clone());

        let parsed = ident_from_type(&idents.single_inner_ty());
        let inner_how_to = parsed.second_how_to().expect("Option needs an inner type");
        let inner_type = match inner_how_to {
            HowToControlThis::WithType(c) => LazyFieldType(c).to_token(),
            HowToControlThis::WithRecurse(RecursiveControlType::Struct) => {
                let t = parsed.internal_type();
                let n = update_to_lazy_ident(t);
                quote! { #n }
            }
            HowToControlThis::WithRecurse(RecursiveControlType::StructLazy) => {
                let t = parsed.internal_type();
                quote! { #t }
            }
            e => panic!("(lazy, enum variant Option) inner not supported: {:?}", e),
        };

        let for_struct = quote! { #variant_ident(Option<#inner_type>) };
        let for_world = quote! {
            #new_enum_ident::#variant_ident(s) => #name::#variant_ident(
                s.as_ref().map(|x| x.eval_lazy(ctx)).transpose()?
            )
        };
        let for_more_defs = quote! {
            #new_enum_ident::#variant_ident(s) => #new_enum_ident::#variant_ident(
                s.as_ref().map(|x| x.with_more_defs(ctx)).transpose()?
            )
        };

        FieldTokensLazy {
            for_struct,
            for_world,
            for_more_defs,
        }
    }
}
