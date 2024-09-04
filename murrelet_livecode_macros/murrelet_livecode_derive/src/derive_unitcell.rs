use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::parser::*;

pub(crate) fn update_to_unitcell_ident(name: syn::Ident) -> syn::Ident {
    prefix_ident("UnitCell", name)
}

pub struct UnitCellFieldType(ControlType);

impl UnitCellFieldType {
    fn to_token(&self) -> TokenStream2 {
        match self.0 {
            ControlType::F32 => quote! {murrelet_livecode::unitcells::UnitCellControlExprF32},
            ControlType::Bool => quote! {murrelet_livecode::unitcells::UnitCellControlExprF32}, // we'll just check if it's above 0
            ControlType::F32_2 => {
                quote! {[murrelet_livecode::unitcells::UnitCellControlExprF32; 2]}
            }
            ControlType::F32_3 => {
                quote! {[murrelet_livecode::unitcells::UnitCellControlExprF32; 3]}
            }
            // ControlType::F32_3 => quote!{[murrelet_livecode::livecode::ControlF32; 3]},
            ControlType::Color => {
                quote! {[murrelet_livecode::unitcells::UnitCellControlExprF32; 4]}
            }
            ControlType::LazyNodeF32 => {
                quote! { murrelet_livecode::lazy::LazyNodeF32Def }
            }
            // ControlType::LinSrgbaUnclamped => quote!{[murrelet_livecode::livecode::ControlF32; 4]},
            _ => panic!("unitcell doesn't have this one yet"),
        }
    }

    fn for_world(&self, idents: StructIdents) -> TokenStream2 {
        let name = idents.name();
        let orig_ty = idents.orig_ty();
        match self.0 {
            ControlType::F32_2 => {
                quote! {#name: glam::vec2(self.#name[0].eval(ctx)? as f32, self.#name[1].eval(ctx)? as f32)}
            }
            // ControlType::F32_3 => quote!{#name: murrelet_livecode::livecode::ControlF32::vec3(&self.#name, w)},
            ControlType::Color => {
                quote! {#name: murrelet_common::MurreletColor::hsva(self.#name[0].eval(ctx)? as f32, self.#name[1].eval(ctx)? as f32, self.#name[2].eval(ctx)? as f32, self.#name[3].eval(ctx)? as f32)}
            }
            // ControlType::LinSrgbaUnclamped => quote!{#name: murrelet_livecode::livecode::ControlF32::hsva_unclamped(&self.#name, w)},
            ControlType::Bool => quote! {#name: self.#name.eval(ctx)? > 0.0},
            // _ => quote!{#name: self.#name.eval(ctx)? as #orig_ty}
            _ => {
                // for number-like things, we also enable clamping! (it's a bit experimental though, be careful)
                let f32_out = match (idents.data.f32min, idents.data.f32max) {
                    (None, None) => quote! {self.#name.eval(ctx)?},
                    (None, Some(max)) => quote! {f32::min(self.#name.eval(ctx)?, #max)},
                    (Some(min), None) => quote! {f32::max(#min, self.#name.eval(ctx)?)},
                    (Some(min), Some(max)) => {
                        quote! {f32::min(f32::max(#min, self.#name.eval(ctx)?), #max)}
                    }
                };
                quote! {#name: #f32_out as #orig_ty}
            }
        }
    }

    fn to_inverted_world(&self, idents: StructIdents) -> TokenStream2 {
        let name = idents.name();
        quote! { #name: self.#name.to_unitcell_input() }
    }

    fn for_newtype_world(&self, idents: StructIdents) -> TokenStream2 {
        let orig_ty = idents.orig_ty();
        match self.0 {
            ControlType::F32_2 => {
                quote! {vec2(self.0[0].eval(ctx)? as f32, self.0[1].eval(ctx)? as f32)}
            }
            // ControlType::F32_3 => quote!{murrelet_livecode::livecode::ControlF32::vec3(&self.0, w)},
            ControlType::Color => {
                quote! {MurreletColor::hsva(self.0[0].eval(ctx)? as f32, self.0[1].eval(ctx)? as f32, self.0[2].eval(ctx)? as f32, self.0[3].eval(ctx)? as f32)}
            }
            // ControlType::LinSrgbaUnclamped => quote!{murrelet_livecode::livecode::ControlF32::hsva_unclamped(&self.0, w)},
            ControlType::Bool => quote! {self.0.eval(ctx)? > 0.0},
            // _ => quote!{self.0.eval(ctx)? as #orig_ty}
            _ => {
                let f32_out = match (idents.data.f32min, idents.data.f32max) {
                    (None, None) => quote! {self.0.eval(ctx)?},
                    (None, Some(max)) => quote! {f32::min(self.0.eval(ctx)?, #max)},
                    (Some(min), None) => quote! {f32::max(#min, self.0.eval(ctx)?)},
                    (Some(min), Some(max)) => {
                        quote! {f32::min(f32::max(#min, self.0.eval(ctx)?), #max)}
                    }
                };
                quote! {#f32_out as #orig_ty}
            }
        }
    }

    fn to_newtype_inverted_world(&self, _idents: StructIdents) -> TokenStream2 {
        quote! { self.0.to_unitcell_input() }
    }
}

pub(crate) struct FieldTokensUnitCell {
    pub(crate) for_struct: TokenStream2,
    pub(crate) for_world: TokenStream2,
    pub(crate) for_inverted_world: TokenStream2, // reversing for_world
}
impl GenFinal for FieldTokensUnitCell {
    fn make_newtype_struct_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensUnitCell>,
    ) -> TokenStream2 {
        let lc_ident = idents.new_ident;
        let name = idents.name;
        let vis = idents.vis;

        let for_struct = variants.iter().map(|x| x.for_struct.clone());
        let for_world = variants.iter().map(|x| x.for_world.clone());
        let for_inverted_world = variants.iter().map(|x| x.for_inverted_world.clone());

        quote! {
            #[derive(Debug, Clone, serde::Deserialize)]
            #vis struct #lc_ident(#(#for_struct,)*);

            impl murrelet_livecode::unitcells::EvaluableUnitCell<#name> for #lc_ident {
                fn eval(&self, ctx: &murrelet_livecode::state::LivecodeWorldState) -> murrelet_livecode::types::LivecodeResult<#name> {
                    Ok(#name(#(#for_world,)*))
                }
            }

            impl murrelet_livecode::unitcells::InvertedWorld<#lc_ident> for #name {
                fn to_unitcell_input(&self) -> #lc_ident {
                    #lc_ident(#(#for_inverted_world,)*)
                }
            }
        }
    }

    fn make_struct_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensUnitCell>,
    ) -> TokenStream2 {
        let lc_ident = idents.new_ident;
        let name = idents.name;
        let vis = idents.vis;

        let for_struct = variants.iter().map(|x| x.for_struct.clone());
        let for_world = variants.iter().map(|x| x.for_world.clone());
        let for_inverted_world = variants.iter().map(|x| x.for_inverted_world.clone());

        quote! {
            #[derive(Debug, Clone, serde::Deserialize)]
            #vis struct #lc_ident {
                #(#for_struct,)*
            }

            impl murrelet_livecode::unitcells::EvaluableUnitCell<#name> for #lc_ident {
                fn eval(&self, ctx: &murrelet_livecode::state::LivecodeWorldState) -> murrelet_livecode::types::LivecodeResult<#name> {
                    Ok(#name {
                        #(#for_world,)*
                    })
                }
            }

            impl murrelet_livecode::unitcells::InvertedWorld<#lc_ident> for #name {
                fn to_unitcell_input(&self) -> #lc_ident {
                    #lc_ident {
                        #(#for_inverted_world,)*
                    }
                }
            }
        }
    }

    fn make_enum_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensUnitCell>,
    ) -> TokenStream2 {
        let new_enum_ident = idents.new_ident;
        let name = idents.name;

        let for_struct = variants.iter().map(|x| x.for_struct.clone());
        let for_world = variants.iter().map(|x| x.for_world.clone());
        let for_inverted_world = variants.iter().map(|x| x.for_inverted_world.clone());

        // #[serde(tag = "type")] replaced with idents
        let enum_tag = idents.tags;

        quote! {
            #[derive(Debug, Clone, serde::Deserialize)]
            #[allow(non_camel_case_types)]
            #enum_tag
            pub enum #new_enum_ident {
                #(#for_struct,)*
            }

            impl murrelet_livecode::unitcells::EvaluableUnitCell<#name> for #new_enum_ident {
                fn eval(&self, ctx: &murrelet_livecode::state::LivecodeWorldState) -> murrelet_livecode::types::LivecodeResult<#name> {
                    Ok(match self {
                        #(#for_world,)*
                    })
                }
            }

            impl murrelet_livecode::unitcells::InvertedWorld<#new_enum_ident> for #name {
                fn to_unitcell_input(&self) -> #new_enum_ident {
                    match self {
                        #(#for_inverted_world,)*
                    }
                }
            }
        }
    }

    fn from_newtype_struct(
        idents: StructIdents,
        _parent_idents: syn::Ident,
    ) -> FieldTokensUnitCell {
        let _serde = idents.serde(true);
        let ctrl = idents.control_type();

        let for_struct = {
            let t = UnitCellFieldType(ctrl).to_token();
            quote! {#t}
        };

        let for_world = UnitCellFieldType(ctrl).for_newtype_world(idents.clone());
        let for_inverted_world = UnitCellFieldType(ctrl).to_newtype_inverted_world(idents.clone());

        FieldTokensUnitCell {
            for_struct,
            for_world,
            for_inverted_world,
        }
    }

    // Arc(CurveArc)
    fn from_unnamed_enum(idents: EnumIdents) -> FieldTokensUnitCell {
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
            let data_from_type = ident_from_type(&t);
            let new_type = update_to_unitcell_ident(data_from_type.main_type);
            quote! { #variant_ident(#new_type) }
        };

        // for world
        let for_world =
            quote! { #new_enum_ident::#variant_ident(s) => #name::#variant_ident(s.eval(ctx)?) };

        let for_inverted_world = quote! { #name::#variant_ident(s) => #new_enum_ident::#variant_ident(s.to_unitcell_input()) };

        FieldTokensUnitCell {
            for_struct,
            for_world,
            for_inverted_world,
        }
    }

    fn from_unit_enum(idents: EnumIdents) -> FieldTokensUnitCell {
        let variant_ident = idents.variant_ident();
        let name = idents.enum_ident();
        let new_enum_ident = Self::new_ident(name.clone());

        let for_struct = {
            quote! { #variant_ident }
        };
        let for_world: TokenStream2 = {
            quote! { #new_enum_ident::#variant_ident => #name::#variant_ident }
        };
        let for_inverted_world: TokenStream2 = {
            quote! { #name::#variant_ident => #new_enum_ident::#variant_ident }
        };

        FieldTokensUnitCell {
            for_struct,
            for_world,
            for_inverted_world,
        }
    }

    fn from_noop_struct(idents: StructIdents) -> FieldTokensUnitCell {
        let serde = idents.serde(true);
        let name = idents.name();
        let new_ty = idents.orig_ty();

        let for_struct = {
            quote! {#serde #name: #new_ty}
        };
        let for_world: TokenStream2 = {
            quote! {#name: self.#name.clone()}
        };
        let for_inverted_world: TokenStream2 = {
            quote! {#name: self.#name.clone()}
        };

        FieldTokensUnitCell {
            for_struct,
            for_world,
            for_inverted_world,
        }
    }

    fn from_type_struct(idents: StructIdents) -> FieldTokensUnitCell {
        let serde = idents.serde(true);
        let name = idents.name();

        let ctrl = idents.control_type();

        let for_struct = {
            let t = UnitCellFieldType(ctrl).to_token();
            quote! {#serde #name: #t}
        };

        let for_world = UnitCellFieldType(ctrl).for_world(idents.clone());
        let for_inverted_world = UnitCellFieldType(ctrl).to_inverted_world(idents.clone());

        FieldTokensUnitCell {
            for_struct,
            for_world,
            for_inverted_world,
        }
    }

    // Vec<CurveSegment>
    fn from_recurse_struct_vec(idents: StructIdents) -> FieldTokensUnitCell {
        let serde = idents.serde(true);
        let name = idents.name();
        let orig_ty = idents.orig_ty();

        let (for_struct, _inside_type) = {
            let target_type = if let DataFromType {
                second_type: Some(second_ty_ident),
                ..
            } = ident_from_type(&orig_ty)
            {
                second_ty_ident
            } else {
                panic!("vec missing second type");
            };

            let infer = HowToControlThis::from_type_str(target_type.clone().to_string().as_ref());

            let src_type = match infer {
                HowToControlThis::WithType(_, c) => UnitCellFieldType(c).to_token(),
                HowToControlThis::WithRecurse(_, RecursiveControlType::Struct) => {
                    let name = Self::new_ident(target_type.clone());
                    quote! {#name}
                }
                HowToControlThis::WithNone(_) => {
                    let name = Self::new_ident(target_type.clone());
                    quote! {#name}
                }
                e => panic!("need vec something {:?}", e),
            };
            (
                quote! {#serde #name: Vec<murrelet_livecode::types::ControlVecElement<#src_type>>},
                infer,
            )
        };
        let debug_name = name.to_string();
        let for_world = {
            quote! {#name: self.#name.iter().map(|x| x.eval_and_expand_vec_for_unitcell(ctx, #debug_name)).collect::<Result<Vec<_>, _>>()?.into_iter().flatten().collect::<Vec<_>>()}
        };

        let for_inverted_world = {
            quote! {#name: self.#name.iter().map(|x| murrelet_livecode::types::ControlVecElement::raw(x.to_unitcell_input())).collect::<Vec<_>>()}
        };

        FieldTokensUnitCell {
            for_struct,
            for_world,
            for_inverted_world,
        }
    }

    fn from_newtype_recurse_struct_vec(idents: StructIdents) -> Self {
        let serde = idents.serde(true);

        let orig_ty = idents.orig_ty();

        let for_struct = {
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
                        HowToControlThis::WithType(_, c) => UnitCellFieldType(c).to_token(),
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
            quote! {#serde #new_ty}
        };
        let for_world = {
            quote! {self.0.iter().map(|x| x.eval(ctx)).collect::<Result<Vec<_>, _>>()?}
        };

        let for_inverted_world = {
            quote! {self.0.iter().map(|x| x.to_unitcell_input()).collect::<Vec<_>>()}
        };

        FieldTokensUnitCell {
            for_struct,
            for_world,
            for_inverted_world,
        }
    }

    fn from_recurse_struct_unitcell(idents: StructIdents) -> FieldTokensUnitCell {
        // transition from livecode to unitcell
        let serde = idents.serde(true);
        let name = idents.name();
        let orig_ty = idents.orig_ty();

        let for_struct: TokenStream2 = {
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
                            // this one has to be unitcell!
                            let name = update_to_unitcell_ident(second_ty_ident.clone());
                            quote! {#name}
                        }

                        e => panic!("need unitcell something {:?}", e),
                    }
                } else {
                    panic!("unitcell missing second type")
                };

                quote! {#ref_lc_ident}
            };

            quote! {#serde #name: #new_ty}
        };

        // to convert it, first grab the config it belongs to, and then run the metrics
        let maybe_target = idents.data.src;
        // should have a value
        let target_name =
            maybe_target.unwrap_or_else(|| panic!("UnitCell {:?} missing src!", name.to_string()));
        let target = syn::Ident::new(&target_name, name.span());

        let maybe_more_ctx = idents
            .data
            .ctx
            .map(|ctx_field| {
                let id = syn::Ident::new(&ctx_field, name.span());
                quote! { Some(self.#id.clone()) }
            })
            .unwrap_or(quote! {None});

        let prefix = idents
            .data
            .prefix
            .map(|ctx_field| {
                quote! { #ctx_field }
            })
            .unwrap_or(quote! {""});

        let for_world = {
            quote! {#name: {
                murrelet_livecode::unitcells::TmpUnitCells::new(
                    self.#target.eval(ctx)?,
                    Box::new(self.#name.clone()),
                    #maybe_more_ctx,
                    #prefix
                ).o(ctx)?
            }}
        };

        let for_inverted_world = {
            quote! {#name: {
                // watch out, this will hardcode every value with the first one
                self.#name.iter().next().cloned().unwrap_or_default().node.to_unitcell_input()
            }}
        };

        FieldTokensUnitCell {
            for_struct,
            for_world,
            for_inverted_world,
        }
    }

    fn from_recurse_struct_struct(idents: StructIdents) -> Self {
        let serde = idents.serde(true);
        let name = idents.name();
        let orig_ty = idents.orig_ty();

        let for_struct = {
            let new_ty = {
                let DataFromType { main_type, .. } = ident_from_type(&orig_ty);
                // let ref_lc_ident = idents.config.new_ident(main_type.clone());
                let ref_lc_ident = Self::new_ident(main_type.clone());

                quote! {#ref_lc_ident}
            };

            quote! {#serde #name: #new_ty}
        };
        let for_world = {
            quote! {#name: self.#name.eval(ctx)?}
        };
        let for_inverted_world = {
            quote! {#name: self.#name.to_unitcell_input()}
        };

        FieldTokensUnitCell {
            for_struct,
            for_world,
            for_inverted_world,
        }
    }

    fn new_ident(name: syn::Ident) -> syn::Ident {
        update_to_unitcell_ident(name)
    }
}
