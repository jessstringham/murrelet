use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::derive_unitcell::update_to_unitcell_ident;
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
            ControlType::EvalExpr => quote! {murrelet_livecode::expr::ControlExprF32},
            ControlType::LazyNodeF32 => quote! {murrelet_livecode::unitcells::LazyNodeF32Def},
        }
    }

    pub(crate) fn for_world(&self, idents: StructIdents) -> TokenStream2 {
        let name = idents.name();
        let orig_ty = idents.orig_ty();
        match self.0 {
            ControlType::F32_2 => quote! {#name: self.#name.o(w)},
            ControlType::F32_3 => quote! {#name: self.#name.o(w)},
            ControlType::Color => quote! {#name: self.#name.o(w)},
            ControlType::ColorUnclamped => {
                quote! {#name: murrelet_livecode::livecode::ControlF32::hsva_unclamped(&self.#name, w)}
            }
            ControlType::LazyNodeF32 => quote! {#name: self.#name.o(w)},
            _ => {
                let f32_out = match (idents.data.f32min, idents.data.f32max) {
                    (None, None) => quote! {self.#name.o(w)},
                    (None, Some(max)) => quote! {f32::min(self.#name.o(w), #max)},
                    (Some(min), None) => quote! {f32::max(#min, self.#name.o(w))},
                    (Some(min), Some(max)) => {
                        quote! {f32::min(f32::max(#min, self.#name.o(w)), #max)}
                    }
                };
                quote! {#name: #f32_out as #orig_ty}
            }
        }
    }

    pub(crate) fn for_newtype_world(&self, idents: StructIdents) -> TokenStream2 {
        let orig_ty = idents.orig_ty();
        match self.0 {
            ControlType::F32_2 => quote! { self.0.o(&w) },
            ControlType::F32_3 => quote! { self.0.o(&w) },
            ControlType::Color => quote! { self.0.o(&w) },
            ControlType::LazyNodeF32 => quote! { self.0.o(&w) },
            ControlType::ColorUnclamped => {
                quote! {murrelet_livecode::livecode::ControlF32::hsva_unclamped(&self.0, w)}
            }
            _ => {
                let f32_out = match (idents.data.f32min, idents.data.f32max) {
                    (None, None) => quote! {self.0.o(w)},
                    (None, Some(max)) => quote! {f32::min(self.0.o(w), #max)},
                    (Some(min), None) => quote! {f32::max(#min, self.0.o(w))},
                    (Some(min), Some(max)) => quote! {f32::min(f32::max(#min, self.0.o(w)), #max)},
                };
                quote! {#f32_out as #orig_ty}
            }
        }
    }

    pub(crate) fn for_timeless_world(&self, idents: StructIdents) -> TokenStream2 {
        let name = idents.name();
        let orig_ty = idents.orig_ty();

        match self.0 {
            //ControlType::F32_2 => quote! {#name: self.#name.just_midi(m) },
            //ControlType::F32_3 => quote! {#name: self.#name.just_midi(m) },
            //ControlType::LinSrgba => quote! {#name:self.#name.just_midi(m) },
            ControlType::ColorUnclamped => {
                quote! {#name: murrelet_livecode::livecode::ControlF32::hsva_unclamped_midi(&self.#name, m)}
            }
            _ => quote! {#name: self.#name.just_midi(m) as #orig_ty},
        }
    }

    pub(crate) fn for_timeless_newtype_world(&self, idents: StructIdents) -> TokenStream2 {
        let orig_ty = idents.orig_ty();
        match self.0 {
            // ControlType::F32_2 => quote! {murrelet_livecode::livecode::ControlF32::vec2_midi(&self.0, m)},
            // ControlType::F32_3 => quote! {murrelet_livecode::livecode::ControlF32::vec3_midi(&self.0, m)},
            // ControlType::LinSrgba => quote! {murrelet_livecode::livecode::ControlF32::hsva_midi(&self.0, m)},
            ControlType::F32_2 => quote! { self.0.just_midi(&m) },
            ControlType::F32_3 => quote! { self.0.just_midi(&m) },
            ControlType::Color => quote! { self.0.just_midi(&m) },
            ControlType::LazyNodeF32 => quote! { self.0.just_midi(&w) },
            ControlType::ColorUnclamped => {
                quote! {murrelet_livecode::livecode::ControlF32::hsva_unclamped_midi(&self.0, m)}
            }
            _ => quote! {self.0.just_midi(m) as #orig_ty},
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
    pub(crate) for_timeless_world: TokenStream2,
    pub(crate) for_to_control: TokenStream2, // a way to convert from original to control
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
        let for_timeless_world = variants.iter().map(|x| x.for_timeless_world.clone());
        let for_to_control = variants.iter().map(|x| x.for_to_control.clone());

        quote! {
            #[derive(Debug, Clone, serde::Deserialize)]
            #vis struct #new_ident {
                #(#for_struct,)*
            }

            impl murrelet_livecode::livecode::LivecodeFromWorld<#name> for #new_ident {
                fn o(&self, w: &murrelet_livecode::livecode::LiveCodeWorldState) -> #name {
                    #name {
                        #(#for_world,)*
                    }
                }

                fn just_midi(&self, m: &murrelet_livecode::livecode::TimelessLiveCodeWorldState) -> #name {
                    #name {
                        #(#for_timeless_world,)*
                    }
                }
            }

            impl murrelet_livecode::livecode::LivecodeToControl<#new_ident> for #name {
                fn to_control(&self) -> #new_ident {
                    #new_ident {
                        #(#for_to_control,)*
                    }
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
        let for_timeless_world = variants.iter().map(|x| x.for_timeless_world.clone());
        let for_to_control = variants.iter().map(|x| x.for_to_control.clone());

        let enum_tag = idents.tags;

        quote! {
            #[derive(Debug, Clone, serde::Deserialize)]
            #[allow(non_camel_case_types)]
            #enum_tag
            #vis enum #new_ident {
                #(#for_struct,)*
            }
            impl murrelet_livecode::livecode::LivecodeFromWorld<#name> for #new_ident {
                fn o(&self, w: &murrelet_livecode::livecode::LiveCodeWorldState) -> #name {
                    match self {
                        #(#for_world,)*
                    }
                }

                fn just_midi(&self, m: &murrelet_livecode::livecode::TimelessLiveCodeWorldState) -> #name {
                    match self {
                        #(#for_timeless_world,)*
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
        let for_timeless_world = variants.iter().map(|x| x.for_timeless_world.clone());
        let for_to_control = variants.iter().map(|x| x.for_to_control.clone());

        quote! {
            #[derive(Debug, Clone, serde::Deserialize)]
            #vis struct #new_ident(#(#for_struct,)*);

            impl murrelet_livecode::livecode::LivecodeFromWorld<#name> for #new_ident {
                fn o(&self, w: &murrelet_livecode::livecode::LiveCodeWorldState) -> #name {
                    #name(#(#for_world,)*)
                }

                fn just_midi(&self, m: &murrelet_livecode::livecode::TimelessLiveCodeWorldState) -> #name {
                    #name(#(#for_timeless_world,)*)
                }
            }

            impl murrelet_livecode::livecode::LivecodeToControl<#new_ident> for #name {
                fn to_control(&self) -> #new_ident {
                    #new_ident(#(#for_to_control,)*)
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

        let for_timeless_world = LivecodeFieldType(ctrl).for_timeless_newtype_world(idents.clone());

        let for_to_control =
            LivecodeFieldType(ctrl).for_newtype_control(idents.clone(), parent_ident.clone());

        FieldTokensLivecode {
            for_struct,
            for_world,
            for_timeless_world,
            for_to_control,
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
        let for_world = quote! { #new_ident::#variant_ident(s) => #name::#variant_ident(s.o(w)) };

        // for timeless world
        let for_timeless_world =
            quote! { #new_ident::#variant_ident(s) => #name::#variant_ident(s.just_midi(m)) };

        let for_to_control =
            quote! { #name::#variant_ident(s) => #new_ident::#variant_ident(s.to_control()) };

        FieldTokensLivecode {
            for_struct,
            for_world,
            for_timeless_world,
            for_to_control,
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
            quote! { #new_ident::#variant_ident => #name::#variant_ident }
        };
        let for_timeless_world = for_world.clone();
        let for_to_control = {
            quote! { #name::#variant_ident => #new_ident::#variant_ident }
        };

        FieldTokensLivecode {
            for_struct,
            for_world,
            for_timeless_world,
            for_to_control,
        }
    }

    fn from_noop_struct(idents: StructIdents) -> FieldTokensLivecode {
        let serde = idents.serde(false);
        let name = idents.name();
        let new_ty = idents.orig_ty();

        let for_struct = {
            quote! {#serde #name: #new_ty}
        };
        let for_world: TokenStream2 = {
            quote! {#name: self.#name.clone()}
        };
        let for_timeless_world = for_world.clone();
        let for_to_control = quote! {#name: self.#name.clone()};

        FieldTokensLivecode {
            for_struct,
            for_world,
            for_timeless_world,
            for_to_control,
        }
    }

    fn from_type_struct(idents: StructIdents) -> FieldTokensLivecode {
        let serde = idents.serde(false).clone();
        let name = idents.name().clone();
        // let _orig_type = idents.orig_ty().clone();

        let ctrl = idents.control_type();
        let for_struct = {
            let t = LivecodeFieldType(ctrl).to_token();
            quote! {#serde #name: #t}
        };
        let for_world = LivecodeFieldType(ctrl).for_world(idents.clone());

        let for_timeless_world = LivecodeFieldType(ctrl).for_timeless_world(idents.clone());

        let for_to_control = LivecodeFieldType(ctrl).for_control(idents.clone());

        FieldTokensLivecode {
            for_struct,
            for_world,
            for_timeless_world,
            for_to_control,
        }
    }

    fn from_recurse_struct_vec(idents: StructIdents) -> FieldTokensLivecode {
        let serde = idents.serde(false);
        let name = idents.name();
        let orig_ty = idents.orig_ty();

        let (for_struct, should_o) = {
            let (new_ty, should_o) = {
                let (ref_lc_ident, should_o) = if let DataFromType {
                    second_type: Some(second_ty_ident),
                    ..
                } = ident_from_type(&orig_ty)
                {
                    let infer = HowToControlThis::from_type_str(
                        second_ty_ident.clone().to_string().as_ref(),
                    );

                    match infer {
                        HowToControlThis::WithType(_, c) => (LivecodeFieldType(c).to_token(), true),
                        HowToControlThis::WithRecurse(_, RecursiveControlType::Struct) => {
                            // let name = idents.config.new_ident(second_ty_ident.clone());
                            let name = Self::new_ident(second_ty_ident.clone());
                            (quote! {#name}, true)
                        }
                        HowToControlThis::WithNone(_) => {
                            // let name = idents.config.new_ident(second_ty_ident.clone());
                            (quote! {#second_ty_ident}, false)
                        }
                        e => panic!("need vec something {:?}", e),
                    }
                } else {
                    panic!("vec missing second type");
                };

                (quote! {Vec<#ref_lc_ident>}, should_o)
            };
            (quote! {#serde #name: #new_ty}, should_o)
        };
        let for_world = {
            if should_o {
                quote! {#name: self.#name.iter().map(|x| x.o(w)).collect::<Vec<_>>()}
            } else {
                quote! {#name: self.#name.clone()}
            }
        };
        let for_timeless_world = {
            if should_o {
                quote! {#name: self.#name.iter().map(|x| x.just_midi(m)).collect::<Vec<_>>()}
            } else {
                quote! {#name: self.#name.clone()}
            }
        };

        let for_to_control = {
            if should_o {
                quote! {#name: self.#name.iter().map(|x| x.to_control()).collect::<Vec<_>>()}
            } else {
                quote! {#name: self.#name.clone()}
            }
        };

        FieldTokensLivecode {
            for_struct,
            for_world,
            for_timeless_world,
            for_to_control,
        }
    }

    fn from_recurse_struct_struct(idents: StructIdents) -> FieldTokensLivecode {
        let serde = idents.serde(false);
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
            quote! {#name: self.#name.o(w)}
        };
        let for_timeless_world = {
            quote! {#name: self.#name.just_midi(m)}
        };
        let for_to_control = {
            quote! {#name: self.#name.to_control()}
        };

        FieldTokensLivecode {
            for_struct,
            for_world,
            for_timeless_world,
            for_to_control,
        }
    }

    fn from_recurse_struct_unitcell(idents: StructIdents) -> FieldTokensLivecode {
        // transition from livecode to unitcell
        let serde = idents.serde(false);
        let name = idents.name();
        let orig_ty = idents.orig_ty();

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

            (quote! {#serde #name: #new_ty}, new_ty.clone())
        };

        // to convert it, first grab the config it belongs to, and then run the metrics
        let maybe_target = idents.data.src; //get_field_from_attrs(orig_attrs, "src");
                                            // should have a value
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
            quote! {#name: {
                murrelet_livecode::unitcells::TmpUnitCells::new(
                    self.#target.o(w),
                    Box::new(self.#name.clone()),
                    #maybe_more_ctx,
                    #prefix
                ).o(w)
            }}
        };
        let for_timeless_world = {
            quote! {#name: {
                murrelet_livecode::unitcells::TmpUnitCells::new(
                    self.#target.just_midi(m),
                    Box::new(self.#name.clone()),
                    #maybe_more_ctx,
                    #prefix
                ).just_midi(m)
            }}
        };

        let for_to_control = {
            quote! {#name: {
                // watch out, this will hardcode every value with the first one
                // also how can i make sure we never drop to 0 items...
                // self.#name.iter().next().map(|x| x.node.to_unitcell_input()).unwrap_or(#new_ty::default())
                self.#name.iter().next().unwrap().node.to_unitcell_input()
            }}
        };

        FieldTokensLivecode {
            for_struct,
            for_world,
            for_timeless_world,
            for_to_control,
        }
    }

    fn from_newtype_recurse_struct_vec(idents: StructIdents) -> Self {
        let serde = idents.serde(false);
        // let name = idents.name();
        let orig_ty = idents.orig_ty();

        let (for_struct, should_o) = {
            let (new_ty, should_o) = {
                let (ref_lc_ident, should_o) = if let DataFromType {
                    second_type: Some(second_ty_ident),
                    ..
                } = ident_from_type(&orig_ty)
                {
                    let infer = HowToControlThis::from_type_str(
                        second_ty_ident.clone().to_string().as_ref(),
                    );

                    match infer {
                        HowToControlThis::WithType(_, c) => (LivecodeFieldType(c).to_token(), true),
                        HowToControlThis::WithRecurse(_, RecursiveControlType::Struct) => {
                            // let name = idents.config.new_ident(second_ty_ident.clone());
                            let name = Self::new_ident(second_ty_ident.clone());
                            (quote! {#name}, true)
                        }
                        HowToControlThis::WithNone(_) => {
                            // let name = idents.config.new_ident(second_ty_ident.clone());
                            (quote! {#second_ty_ident}, false)
                        }
                        e => panic!("need vec something {:?}", e),
                    }
                } else {
                    panic!("vec missing second type");
                };

                (quote! {Vec<#ref_lc_ident>}, should_o)
            };
            (quote! {#serde #new_ty}, should_o)
        };
        let for_world = {
            if should_o {
                quote! {self.0.iter().map(|x| x.o(w)).collect::<Vec<_>>()}
            } else {
                quote! {self.0.clone()}
            }
        };
        let for_timeless_world = {
            if should_o {
                quote! {self.0.iter().map(|x| x.just_midi(m)).collect::<Vec<_>>()}
            } else {
                quote! {self.0.clone()}
            }
        };

        let for_to_control = {
            if should_o {
                quote! {self.0.iter().map(|x| x.to_control()).collect::<Vec<_>>()}
            } else {
                quote! {self.0.clone()}
            }
        };

        FieldTokensLivecode {
            for_struct,
            for_world,
            for_timeless_world,
            for_to_control,
        }
    }
}
