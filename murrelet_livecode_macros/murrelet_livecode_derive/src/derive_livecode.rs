use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::parser::*;

pub(crate) fn update_to_control_ident(name: syn::Ident) -> syn::Ident {
    prefix_ident("Control", name)
}

pub(crate) struct LivecodeFieldType(pub ControlType);

impl LivecodeFieldType {
    pub fn to_token(&self) -> TokenStream2 {
        match self.0 {
            ControlType::F32 => quote! {murrelet_livecode::livecode::ControlF32},
            ControlType::Bool => quote! {murrelet_livecode::livecode::ControlBool},
            ControlType::F32_2 => quote! {[murrelet_livecode::livecode::ControlF32; 2]},
            ControlType::F32_3 => quote! {[murrelet_livecode::livecode::ControlF32; 3]},
            ControlType::Color => quote! {[murrelet_livecode::livecode::ControlF32; 4]},
            ControlType::ColorUnclamped => quote! {[murrelet_livecode::livecode::ControlF32; 4]},
            // ControlType::EvalExpr => quote! {murrelet_livecode::expr::ControlExprF32},
            ControlType::LazyNodeF32 => quote! {murrelet_livecode::lazy::ControlLazyNodeF32},
        }
    }

    // usually can call for_world directly, but this is useful in Vec<>
    pub(crate) fn for_world_no_name(
        &self,
        name: syn::Ident,
        orig_ty: syn::Type,
        f32min: Option<f32>,
        f32max: Option<f32>,
    ) -> TokenStream2 {
        // let name = idents.name();
        // let orig_ty = idents.orig_ty();
        match self.0 {
            ControlType::F32_2 => quote! {self.#name.o(w)?},
            ControlType::F32_3 => quote! {self.#name.o(w)?},
            ControlType::Color => quote! {self.#name.o(w)?},
            ControlType::ColorUnclamped => {
                quote! {murrelet_livecode::livecode::ControlF32::hsva_unclamped(&self.#name, w)?}
            }
            ControlType::LazyNodeF32 => quote! {self.#name.o(w)?},
            _ => {
                let f32_out = match (f32min, f32max) {
                    (None, None) => quote! {self.#name.o(w)?},
                    (None, Some(max)) => quote! {f32::min(self.#name.o(w)?, #max)},
                    (Some(min), None) => quote! {f32::max(#min, self.#name.o(w)?)},
                    (Some(min), Some(max)) => {
                        quote! {f32::min(f32::max(#min, self.#name.o(w)?), #max)}
                    }
                };
                quote! {#f32_out as #orig_ty}
            }
        }
    }

    pub(crate) fn for_world(&self, idents: StructIdents) -> TokenStream2 {
        let name = idents.name();
        let rest = self.for_world_no_name(
            idents.name(),
            idents.orig_ty(),
            idents.data.f32min,
            idents.data.f32max,
        );
        quote! {#name: #rest}
    }

    pub(crate) fn for_newtype_world(&self, idents: StructIdents) -> TokenStream2 {
        let orig_ty = idents.orig_ty();
        match self.0 {
            ControlType::F32_2 => quote! { self.0.o(&w)? },
            ControlType::F32_3 => quote! { self.0.o(&w)? },
            ControlType::Color => quote! { self.0.o(&w)? },
            ControlType::LazyNodeF32 => quote! { self.0.o(&w)? },
            ControlType::ColorUnclamped => {
                quote! {murrelet_livecode::livecode::ControlF32::hsva_unclamped(&self.0, w)?}
            }
            _ => {
                let f32_out = match (idents.data.f32min, idents.data.f32max) {
                    (None, None) => quote! {self.0.o(w)?},
                    (None, Some(max)) => quote! {f32::min(self.0.o(w)?, #max)},
                    (Some(min), None) => quote! {f32::max(#min, self.0.o(w)?)},
                    (Some(min), Some(max)) => quote! {f32::min(f32::max(#min, self.0.o(w)?), #max)},
                };
                quote! {#f32_out as #orig_ty}
            }
        }
    }

    pub(crate) fn for_control(&self, idents: StructIdents) -> TokenStream2 {
        let name = idents.name();
        quote! { #name: self.#name.to_control() }
    }

    pub(crate) fn for_newtype_control(
        &self,
        _idents: StructIdents,
        _parent_idents: syn::Ident,
    ) -> TokenStream2 {
        quote! { self.0.to_control() }
    }
}

pub(crate) struct FieldTokensLivecode {
    pub(crate) for_struct: TokenStream2,
    pub(crate) for_world: TokenStream2,
    pub(crate) for_to_control: TokenStream2, // a way to convert from original to control
    pub(crate) for_variable_idents: TokenStream2,
    pub(crate) for_function_idents: TokenStream2,
}
impl GenFinal for FieldTokensLivecode {
    fn make_struct_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensLivecode>,
    ) -> TokenStream2 {
        let new_ident = idents.new_ident;
        let name = idents.name;
        let vis = idents.vis;

        let for_struct = variants.iter().map(|x| x.for_struct.clone());
        let for_world = variants.iter().map(|x| x.for_world.clone());
        let for_to_control = variants.iter().map(|x| x.for_to_control.clone());
        let for_variable_idents = variants.iter().map(|x| x.for_variable_idents.clone());
        let for_function_idents = variants.iter().map(|x| x.for_function_idents.clone());

        let maybe_cfg_attr = if cfg!(feature = "schemars") {
            quote! {, schemars::JsonSchema}
        } else {
            quote! {}
        };

        quote! {
            #[derive(Debug, Clone, serde::Deserialize #maybe_cfg_attr)]
            #vis struct #new_ident {
                #(#for_struct,)*
            }

            impl murrelet_livecode::livecode::LivecodeFromWorld<#name> for #new_ident {
                fn o(&self, w: &murrelet_livecode::state::LivecodeWorldState) -> murrelet_livecode::types::LivecodeResult<#name> {
                    Ok(#name {
                        #(#for_world,)*
                    })
                }
            }

            impl murrelet_livecode::livecode::LivecodeToControl<#new_ident> for #name {
                fn to_control(&self) -> #new_ident {
                    #new_ident {
                        #(#for_to_control,)*
                    }
                }
            }

            impl murrelet_livecode::livecode::GetLivecodeIdentifiers for #new_ident {
                fn variable_identifiers(&self) -> Vec<murrelet_livecode::livecode::LivecodeVariable> {
                    vec![#(#for_variable_idents,)*]
                        .concat()
                        .into_iter()
                        .collect::<std::collections::HashSet<_>>()
                        .into_iter()
                        .collect::<Vec<_>>()
                }

                fn function_identifiers(&self) -> Vec<murrelet_livecode::livecode::LivecodeFunction> {
                    vec![#(#for_function_idents,)*]
                        .concat()
                        .into_iter()
                        .collect::<std::collections::HashSet<_>>()
                        .into_iter()
                        .collect::<Vec<_>>()
                }
            }
        }
    }

    fn make_enum_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensLivecode>,
    ) -> TokenStream2 {
        let new_ident = idents.new_ident;
        let vis = idents.vis;
        let name = idents.name;

        let for_struct = variants.iter().map(|x| x.for_struct.clone());
        let for_world = variants.iter().map(|x| x.for_world.clone());
        let for_to_control = variants.iter().map(|x| x.for_to_control.clone());
        let for_variable_idents = variants.iter().map(|x| x.for_variable_idents.clone());
        let for_function_idents = variants.iter().map(|x| x.for_function_idents.clone());

        let enum_tag = idents.tags;

        let maybe_cfg_attr = if cfg!(feature = "schemars") {
            quote! {, schemars::JsonSchema}
        } else {
            quote! {}
        };

        quote! {
            #[derive(Debug, Clone, serde::Deserialize #maybe_cfg_attr)]
            #[allow(non_camel_case_types)]
            #enum_tag
            #vis enum #new_ident {
                #(#for_struct,)*
            }
            impl murrelet_livecode::livecode::LivecodeFromWorld<#name> for #new_ident {
                fn o(&self, w: &murrelet_livecode::state::LivecodeWorldState) -> murrelet_livecode::types::LivecodeResult<#name> {
                    match self {
                        #(#for_world,)*
                    }
                }
            }

            impl murrelet_livecode::livecode::LivecodeToControl<#new_ident> for #name {
                fn to_control(&self) -> #new_ident {
                    match self {
                        #(#for_to_control,)*
                    }
                }
            }

            impl murrelet_livecode::livecode::GetLivecodeIdentifiers for #new_ident {
                fn variable_identifiers(&self) -> Vec<murrelet_livecode::livecode::LivecodeVariable> {
                    match self {
                        #(#for_variable_idents,)*
                    }
                }

                fn function_identifiers(&self) -> Vec<murrelet_livecode::livecode::LivecodeFunction> {
                    match self {
                        #(#for_function_idents,)*
                    }
                }
            }
        }
    }

    fn make_newtype_struct_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensLivecode>,
    ) -> TokenStream2 {
        let new_ident = idents.new_ident;
        let name = idents.name;
        let vis = idents.vis;

        let for_struct = variants.iter().map(|x| x.for_struct.clone());
        let for_world = variants.iter().map(|x| x.for_world.clone());
        let for_to_control = variants.iter().map(|x| x.for_to_control.clone());
        let for_variable_idents = variants.iter().map(|x| x.for_variable_idents.clone());
        let for_function_idents = variants.iter().map(|x| x.for_function_idents.clone());

        let maybe_cfg_attr = if cfg!(feature = "schemars") {
            quote! {, schemars::JsonSchema}
        } else {
            quote! {}
        };

        quote! {
            #[derive(Debug, Clone, serde::Deserialize #maybe_cfg_attr)]
            #vis struct #new_ident(#(#for_struct,)*);

            impl murrelet_livecode::livecode::LivecodeFromWorld<#name> for #new_ident {
                fn o(&self, w: &murrelet_livecode::state::LivecodeWorldState) -> murrelet_livecode::types::LivecodeResult<#name> {
                    Ok(#name(#(#for_world,)*))
                }
            }

            impl murrelet_livecode::livecode::LivecodeToControl<#new_ident> for #name {
                fn to_control(&self) -> #new_ident {
                    #new_ident(#(#for_to_control,)*)
                }
            }

            impl murrelet_livecode::livecode::GetLivecodeIdentifiers for #new_ident {
                fn variable_identifiers(&self) -> Vec<murrelet_livecode::livecode::LivecodeVariable> {
                    #(#for_variable_idents)*
                }

                fn function_identifiers(&self) -> Vec<murrelet_livecode::livecode::LivecodeFunction> {
                    #(#for_function_idents)*
                }
            }
        }
    }

    fn new_ident(name: syn::Ident) -> syn::Ident {
        update_to_control_ident(name)
    }

    fn from_newtype_struct(idents: StructIdents, parent_ident: syn::Ident) -> FieldTokensLivecode {
        // let serde = idents.serde(false).clone();
        // let name = idents.name().clone();
        // let _orig_type = idents.orig_ty().clone();

        let ctrl = idents.control_type();
        let for_struct = {
            let t = LivecodeFieldType(ctrl).to_token();
            quote! {#t}
        };
        let for_world = LivecodeFieldType(ctrl).for_newtype_world(idents.clone());

        let for_to_control =
            LivecodeFieldType(ctrl).for_newtype_control(idents.clone(), parent_ident.clone());

        let (for_variable_idents, for_function_idents) = if idents.how_to_control_this_is_none() {
            (quote! { vec![] }, quote! { vec![] })
        } else {
            (
                quote! {self.0.variable_identifiers()},
                quote! {self.0.function_identifiers()},
            )
        };

        FieldTokensLivecode {
            for_struct,
            for_world,
            for_to_control,
            for_variable_idents,
            for_function_idents,
        }
    }

    fn from_newtype_struct_lazy(idents: StructIdents, parent_ident: syn::Ident) -> Self {
        Self::from_newtype_struct_struct(idents, parent_ident)
    }

    fn from_newtype_struct_struct(
        idents: StructIdents,
        _parent_ident: syn::Ident,
    ) -> FieldTokensLivecode {
        // let serde = idents.serde(false).clone();
        // let name = idents.name().clone();
        // let _orig_type = idents.orig_ty().clone();

        // we need to get the internal struct type
        let orig_ty = idents.orig_ty();
        let parsed_type_info = ident_from_type(&orig_ty);
        let internal_type = parsed_type_info.main_type;

        let for_struct = {
            let t = Self::new_ident(internal_type);
            quote! {#t}
        };
        let for_world = {
            quote! { self.0.o(w)? }
        };

        let for_to_control = {
            quote! { self.0.to_control() }
        };

        let (for_variable_idents, for_function_idents) = if idents.how_to_control_this_is_none() {
            (quote! { vec![] }, quote! { vec![] })
        } else {
            (
                quote! {self.0.variable_identifiers()},
                quote! {self.0.function_identifiers()},
            )
        };

        FieldTokensLivecode {
            for_struct,
            for_world,
            for_to_control,
            for_variable_idents,
            for_function_idents,
        }
    }

    // e.g. TileAxisLocs::V(TileAxisVs)
    fn from_unnamed_enum(idents: EnumIdents) -> FieldTokensLivecode {
        let variant_ident = idents.variant_ident();
        let name = idents.enum_ident();
        let new_ident = Self::new_ident(name.clone());

        let unnamed = idents.data.fields.fields;

        // for struct
        if unnamed.len() != 1 {
            panic!("multiple fields not supported")
        };

        let for_struct = {
            let t = unnamed.first().unwrap().clone().ty;
            let DataFromType { main_type, .. } = ident_from_type(&t);
            let new_type = update_to_control_ident(main_type);
            quote! { #variant_ident(#new_type) }
        };

        // for world
        let for_world =
            quote! { #new_ident::#variant_ident(s) => Ok(#name::#variant_ident(s.o(w)?)) };

        let for_to_control =
            quote! { #name::#variant_ident(s) => #new_ident::#variant_ident(s.to_control()) };

        let for_variable_idents =
            quote! { #new_ident::#variant_ident(s) => s.variable_identifiers() };
        let for_function_idents =
            quote! { #new_ident::#variant_ident(s) => s.function_identifiers() };

        FieldTokensLivecode {
            for_struct,
            for_world,
            for_to_control,
            for_variable_idents,
            for_function_idents,
        }
    }

    // e.g. TileAxis::Diag
    fn from_unit_enum(idents: EnumIdents) -> FieldTokensLivecode {
        let variant_ident = idents.variant_ident();
        let name = idents.enum_ident();
        let new_ident = Self::new_ident(name.clone());

        let for_struct = {
            quote! { #variant_ident }
        };
        let for_world: TokenStream2 = {
            quote! { #new_ident::#variant_ident => Ok(#name::#variant_ident) }
        };
        let for_to_control = {
            quote! { #name::#variant_ident => #new_ident::#variant_ident }
        };

        let for_variable_idents = quote! { #new_ident::#variant_ident => vec![] };
        let for_function_idents = quote! { #new_ident::#variant_ident => vec![] };

        FieldTokensLivecode {
            for_struct,
            for_world,
            for_to_control,
            for_variable_idents,
            for_function_idents,
        }
    }

    fn from_noop_struct(idents: StructIdents) -> FieldTokensLivecode {
        let serde = idents.serde();
        let name = idents.name();
        let new_ty = idents.orig_ty();

        let for_struct = {
            quote! {#serde #name: #new_ty}
        };
        let for_world: TokenStream2 = {
            quote! {#name: self.#name.clone()}
        };
        let for_to_control = quote! {#name: self.#name.clone()};

        let for_variable_idents = quote! { self.#name.variable_identifiers() };
        let for_function_idents = quote! { self.#name.function_identifiers() };

        FieldTokensLivecode {
            for_struct,
            for_world,
            for_to_control,
            for_variable_idents,
            for_function_idents,
        }
    }

    fn from_type_struct(idents: StructIdents) -> FieldTokensLivecode {
        let serde = idents.serde().clone();
        let name = idents.name().clone();
        // let _orig_type = idents.orig_ty().clone();

        let ctrl = idents.control_type();
        let for_struct = {
            let t = LivecodeFieldType(ctrl).to_token();
            quote! {#serde #name: #t}
        };
        let for_world = LivecodeFieldType(ctrl).for_world(idents.clone());

        let for_to_control = LivecodeFieldType(ctrl).for_control(idents.clone());

        // we'll just use the trait (i want to try it for the above, but we'll come back to that!)
        let (for_variable_idents, for_function_idents) = if idents.how_to_control_this_is_none() {
            (quote! { vec![] }, quote! { vec![] })
        } else {
            (
                quote! {self.#name.variable_identifiers()},
                quote! {self.#name.function_identifiers()},
            )
        };

        FieldTokensLivecode {
            for_struct,
            for_world,
            for_to_control,
            for_variable_idents,
            for_function_idents,
        }
    }

    // Vec<CurveSegment>, Vec<f32>
    fn from_recurse_struct_vec(idents: StructIdents) -> FieldTokensLivecode {
        let serde = idents.serde();
        let name = idents.name();
        let orig_ty = idents.orig_ty();

        let parsed_type_info = ident_from_type(&orig_ty);
        let how_to_control_internal = parsed_type_info.how_to_control_internal();
        let wrapper = parsed_type_info.wrapper_type();

        let for_struct = {
            let src_type = match how_to_control_internal {
                HowToControlThis::WithType(_, c) => LivecodeFieldType(*c).to_token(),
                HowToControlThis::WithRecurse(_, RecursiveControlType::Struct) => {
                    let target_type = parsed_type_info.internal_type();
                    let name = Self::new_ident(target_type.clone());
                    quote! {#name}
                }
                HowToControlThis::WithRecurse(_, RecursiveControlType::StructLazy) => {
                    let original_internal_type = parsed_type_info.internal_type();
                    let name = Self::new_ident(original_internal_type.clone());
                    quote! {#name}
                }
                HowToControlThis::WithNone(_) => {
                    let target_type = parsed_type_info.internal_type();
                    quote! {#target_type}
                }
                e => panic!("need vec something {:?}", e),
            };

            let new_ty = match wrapper {
                VecDepth::NotAVec => unreachable!("not a vec in a vec?"),
                VecDepth::Vec => {
                    quote! { Vec<murrelet_livecode::types::ControlVecElement<#src_type>> }
                }
                VecDepth::VecVec => {
                    quote! { Vec<Vec<murrelet_livecode::types::ControlVecElement<#src_type>>> }
                }
            };

            quote! {#serde #name: #new_ty}
        };

        let for_world = {
            if how_to_control_internal.needs_to_be_evaluated() {
                match wrapper {
                    VecDepth::NotAVec => unreachable!("not a vec in a vec?"),
                    VecDepth::Vec => {
                        quote! {
                        #name: self.#name.iter()
                            .map(|x| x.eval_and_expand_vec(w))
                            .collect::<Result<Vec<_>, _>>()?
                            .into_iter()
                            .flatten()
                            .collect()}
                    }
                    VecDepth::VecVec => {
                        quote! {
                            #name: {
                                let mut result = Vec::with_capacity(self.#name.len());
                                for internal_row in &self.#name {
                                    result.push(
                                        internal_row.iter()
                                            .map(|x| x.eval_and_expand_vec(w))
                                            .collect::<Result<Vec<_>, _>>()?
                                            .into_iter()
                                            .flatten()
                                            .collect()
                                    )
                                }
                                result
                            }
                        }
                    }
                }
            } else {
                quote! {#name: self.#name.clone()}
            }

            // match infer {
            //     HowToControlThis::WithType(_, _c) => {
            //         quote! {#name: self.#name.iter().map(|x| x.eval_and_expand_vec(w, #debug_name)).collect::<Result<Vec<_>, _>>()?.into_iter().flatten().collect()}
            //     }
            //     HowToControlThis::WithRecurse(_, _) => {
            //         quote! {#name: self.#name.iter().map(|x| x.eval_and_expand_vec(w, #debug_name)).collect::<Result<Vec<_>, _>>()?.into_iter().flatten().collect()}
            //     }
            //     HowToControlThis::WithNone(_) => {
            //         quote! {#name: self.#name.clone()}
            //     }
            // }
        };

        let for_to_control = {
            if how_to_control_internal.needs_to_be_evaluated() {
                match wrapper {
                    VecDepth::NotAVec => unreachable!("not a vec in a vec?"),
                    VecDepth::Vec => {
                        quote! { #name: self.#name.iter().map(|x| murrelet_livecode::types::ControlVecElement::raw(x.to_control())).collect::<Vec<_>>() }
                    }
                    VecDepth::VecVec => {
                        quote! {
                            #name: {
                                let mut result = Vec::with_capacity(self.#name.len());
                                for internal_row in &self.#name {
                                    result.push(
                                        internal_row.iter().map(|x| murrelet_livecode::types::ControlVecElement::raw(x.to_control())).collect::<Vec<_>>()
                                    )
                                }
                                result
                            }
                        }
                    }
                }
            } else {
                quote! {#name: self.#name.clone()}
            }

            // match infer {
            //     HowToControlThis::WithType(_, _c) => {
            //         quote! {#name: self.#name.iter().map(|x| murrelet_livecode::types::ControlVecElement::raw(x.to_control())).collect::<Vec<_>>()}
            //     }
            //     HowToControlThis::WithRecurse(_, _) => {
            //         quote! {#name: self.#name.iter().map(|x| murrelet_livecode::types::ControlVecElement::raw(x.to_control())).collect::<Vec<_>>()}
            //     }
            //     HowToControlThis::WithNone(_) => {
            //         quote! {#name: self.#name.clone()}
            //     }
            // }
        };

        let for_variable_idents = {
            if how_to_control_internal.needs_to_be_evaluated() {
                match wrapper {
                    VecDepth::NotAVec => unreachable!("not a vec in a vec?"),
                    VecDepth::Vec => {
                        quote! {self.#name.iter().map(|x| x.variable_identifiers()).into_iter().flatten().collect::<std::collections::HashSet<_>>().into_iter().collect::<Vec<_>>()}
                    }
                    VecDepth::VecVec => {
                        quote! {
                            {
                                let mut result = Vec::with_capacity(self.#name.len());
                                for internal_row in &self.#name {
                                    result.extend(
                                        internal_row.iter().map(|x| x.variable_identifiers()).into_iter().flatten().collect::<std::collections::HashSet<_>>().into_iter()
                                    );
                                }
                                result
                            }
                        }
                    }
                }
            } else {
                quote! {vec![]}
            }
        };

        let for_function_idents = {
            if how_to_control_internal.needs_to_be_evaluated() {
                match wrapper {
                    VecDepth::NotAVec => unreachable!("not a vec in a vec?"),
                    VecDepth::Vec => {
                        quote! {self.#name.iter().map(|x| x.function_identifiers()).into_iter().flatten().collect::<std::collections::HashSet<_>>().into_iter().collect::<Vec<_>>()}
                    }
                    VecDepth::VecVec => {
                        quote! {
                            {
                                let mut result = Vec::with_capacity(self.#name.len());
                                for internal_row in &self.#name {
                                    result.extend(
                                        internal_row.iter().map(|x| x.function_identifiers()).into_iter().flatten().collect::<std::collections::HashSet<_>>().into_iter()
                                    );
                                }
                                result
                            }
                        }
                    }
                }
            } else {
                quote! {vec![]}
            }
        };

        FieldTokensLivecode {
            for_struct,
            for_world,
            for_to_control,
            for_variable_idents,
            for_function_idents,
        }
    }

    fn from_recurse_struct_struct(idents: StructIdents) -> FieldTokensLivecode {
        let serde = idents.serde();
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
            quote! {#name: self.#name.o(w)?}
        };
        let for_to_control = {
            quote! {#name: self.#name.to_control()}
        };

        let for_variable_idents = quote! { self.#name.variable_identifiers() };
        let for_function_idents = quote! { self.#name.function_identifiers() };

        FieldTokensLivecode {
            for_struct,
            for_world,
            for_to_control,
            for_variable_idents,
            for_function_idents,
        }
    }

    fn from_recurse_struct_unitcell(idents: StructIdents) -> FieldTokensLivecode {
        // transition from livecode to unitcell
        let serde = idents.serde();
        let name = idents.name();
        let orig_ty = idents.orig_ty();

        let parsed_type_info = ident_from_type(&orig_ty);
        let how_to_control_internal = parsed_type_info.how_to_control_internal();

        let for_struct = {
            let new_ty = match how_to_control_internal {
                HowToControlThis::WithRecurse(_, RecursiveControlType::Struct) => {
                    let internal_type = parsed_type_info.internal_type();
                    let name = update_to_control_ident(internal_type);
                    quote! {#name}
                }

                HowToControlThis::WithRecurse(_, RecursiveControlType::StructLazy) => {
                    let internal_type = parsed_type_info.internal_type();
                    let name = update_to_control_ident(internal_type);
                    quote! {#name}
                }

                e => panic!("need unitcell something {:?}", e),
            };

            quote! {#serde #name: #new_ty}
        };

        // to convert it, first grab the config it belongs to, and then run the metrics
        let maybe_target = idents.data.src;
        let target_name = maybe_target.expect("UnitCells missing src!");
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
            // todo, these look like the same
            if how_to_control_internal.is_lazy() {
                quote! {#name: {
                    murrelet_livecode::unitcells::TmpUnitCells::new(
                        self.#target.o(w)?,
                        Box::new(self.#name.clone()),
                        #maybe_more_ctx,
                        #prefix
                    ).o(&w)? // maybe switch this?
                }}
            } else {
                quote! {#name: {
                    murrelet_livecode::unitcells::TmpUnitCells::new(
                        self.#target.o(w)?,
                        Box::new(self.#name.clone()),
                        #maybe_more_ctx,
                        #prefix
                    ).o(&w)?
                }}
            }
        };

        let for_to_control = {
            if how_to_control_internal.is_lazy() {
                quote! {#name: {
                    // watch out, this will hardcode every value with the first one
                    // also how can i make sure we never drop to 0 items...
                    self.#name.iter().next().unwrap().node.to_control()
                }}
            } else {
                quote! {
                    #name: self.#name.iter().next().unwrap().node.to_control()
                }
            }
        };

        // we just need to grab
        let for_variable_idents = quote! {
            vec![
                self.#target.variable_identifiers(),
                self.#name.variable_identifiers()
            ].concat()
        };
        let for_function_idents = quote! {
            vec![
                self.#target.function_identifiers(),
                self.#name.function_identifiers()
            ].concat()
        };

        FieldTokensLivecode {
            for_struct,
            for_world,
            for_to_control,
            for_variable_idents,
            for_function_idents,
        }
    }

    // Thing(Vec<Something>);
    fn from_newtype_recurse_struct_vec(idents: StructIdents) -> Self {
        let serde = idents.serde();
        let orig_ty = idents.orig_ty();

        let parsed_type_info = ident_from_type(&orig_ty);
        let how_to_control_internal = parsed_type_info.how_to_control_internal();
        let wrapper = parsed_type_info.wrapper_type();

        let for_struct = {
            let internal_type = match how_to_control_internal {
                HowToControlThis::WithType(_, c) => LivecodeFieldType(*c).to_token(),
                HowToControlThis::WithRecurse(_, RecursiveControlType::Struct) => {
                    let original_internal_type = parsed_type_info.internal_type();
                    let name = Self::new_ident(original_internal_type.clone());
                    quote! {#name}
                }
                HowToControlThis::WithRecurse(_, RecursiveControlType::StructLazy) => {
                    let original_internal_type = parsed_type_info.internal_type();
                    let name = Self::new_ident(original_internal_type.clone());
                    quote! {#name}
                }
                HowToControlThis::WithNone(_) => {
                    let original_internal_type = parsed_type_info.internal_type();
                    quote! {#original_internal_type}
                }
                e => panic!("need vec something {:?}", e),
            };

            let new_ty = match wrapper {
                VecDepth::NotAVec => unreachable!("huh, parsing a not-vec in the vec function"), // why is it in this function?
                VecDepth::Vec => quote! {Vec<#internal_type>},
                VecDepth::VecVec => quote! {Vec<Vec<#internal_type>>},
            };
            quote! {#serde #new_ty}
        };
        let for_world = {
            if how_to_control_internal.needs_to_be_evaluated() {
                match wrapper {
                    VecDepth::NotAVec => todo!(),
                    VecDepth::Vec => {
                        quote! {self.0.iter().map(|x| x.o(w)).collect::<Result<Vec<_>, _>>()?}
                    }
                    VecDepth::VecVec => unimplemented!(),
                }
            } else {
                quote! {self.0.clone()}
            }
        };

        let for_to_control = {
            if how_to_control_internal.needs_to_be_evaluated() {
                quote! {self.0.iter().map(|x| x.to_control()).collect::<Vec<_>>()}
            } else {
                quote! {self.0.clone()}
            }
        };

        let for_variable_idents = {
            if how_to_control_internal.needs_to_be_evaluated() {
                quote! {self.0.iter().map(|x| x.variable_identifiers()).flatten().collect::<std::collections::HashSet<_>>().into_iter().collect::<Vec<_>>()}
            } else {
                quote! { vec![] }
            }
        };

        let for_function_idents = {
            if how_to_control_internal.needs_to_be_evaluated() {
                quote! {self.0.iter().map(|x| x.function_identifiers()).flatten().collect::<std::collections::HashSet<_>>().into_iter().collect::<Vec<_>>()}
            } else {
                quote! { vec![] }
            }
        };

        FieldTokensLivecode {
            for_struct,
            for_world,
            for_to_control,
            for_variable_idents,
            for_function_idents,
        }
    }

    fn from_recurse_struct_lazy(idents: StructIdents) -> Self {
        Self::from_recurse_struct_struct(idents)
    }
}
