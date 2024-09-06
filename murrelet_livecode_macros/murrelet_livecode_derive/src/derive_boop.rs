use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::parser::*;

pub(crate) fn update_to_boop_ident(name: syn::Ident) -> syn::Ident {
    prefix_ident("Boop", name)
}

pub(crate) struct BoopFieldType(ControlType);

impl BoopFieldType {
    fn to_token(&self) -> TokenStream2 {
        match self.0 {
            ControlType::F32 => quote! {murrelet_livecode::boop::BoopState},
            ControlType::F32_2 => quote! {murrelet_livecode::boop::BoopState2},
            ControlType::F32_3 => quote! {murrelet_livecode::boop::BoopState3},
            ControlType::Color => quote! {murrelet_livecode::boop::BoopStateHsva},

            // nothing fancy here yet either..
            ControlType::LazyNodeF32 => quote! {murrelet_livecode::lazy::LazyNodeF32},

            // ControlType::LinSrgbaUnclamped => quote!{[murrelet_livecode::livecode::ControlF32; 4]},
            ControlType::Bool => quote! {bool}, // nothing fancy here yet
            _ => panic!("boop doesn't have {:?} yet", self.0),
        }
    }

    fn for_world(&self, idents: StructIdents) -> TokenStream2 {
        let name = idents.name();
        let orig_ty = idents.orig_ty();
        let yaml_name = idents.name().to_string();
        let new_conf = quote! {&conf.copy_with_new_current_boop(#yaml_name)};

        match self.0 {
            ControlType::F32 => {
                quote! {#name: self.#name.boop(&conf.copy_with_new_current_boop(#yaml_name), t, &(target.#name as f32)) as #orig_ty}
            }
            //ControlType::F32_2 => quote!{#name: self.#name[0].boop(#new_conf, t, target.#name), self.#name[1].boop(conf, t, target.#name[1]))},
            // ControlType::F32_3 => quote!{#name: vec3(self.#name[0].boop(conf, t, target.#name[0]), self.#name[1].boop(conf, t, target.#name[1]), self.#name[2].boop(conf, t, target.#name[2]))},
            // ControlType::LinSrgba => quote!{#name: hsva(self.#name[0].boop(conf, t, target.#name[0]), self.#name[1].boop(conf, t, target.#name[1]), self.#name[2].boop(conf, t, target.#name[2]), self.#name[3].boop(conf, t, target.#name[3])).into_lin_srgba()},
            // ControlType::LinSrgbaUnclamped => quote!{#name: murrelet_livecode::livecode::ControlF32::hsva_unclamped(&self.#name)},
            ControlType::Bool => quote! {#name: self.#name.clone()}, // sorry, nothing fancy for bools yet
            ControlType::LazyNodeF32 => quote! {#name: self.#name.clone()}, // sorry, nothing fancy for bools yet

            // _ => quote!{#name: self.#name.boop(conf, t, &target.#name) as #orig_ty} // try to convert back to usize/etc
            _ => {
                let f32_out = match (idents.data.f32min, idents.data.f32max) {
                    (None, None) => quote! {self.#name.boop(#new_conf, t, &target.#name)},
                    (None, Some(max)) => {
                        quote! {f32::min(self.#name.boop(#new_conf, t, &target.#name), #max)}
                    }
                    (Some(min), None) => {
                        quote! {f32::max(#min, self.#name.boop(#new_conf, t, &target.#name))}
                    }
                    (Some(min), Some(max)) => {
                        quote! {f32::min(f32::max(#min, self.#name.boop(#new_conf, t, &target.#name)), #max)}
                    }
                };
                quote! {#name: #f32_out as #orig_ty}
            }
        }
    }

    fn for_boop_init(&self, idents: StructIdents) -> TokenStream2 {
        let name = idents.name();
        // let orig_ty = idents.orig_ty;
        // let new_type = idents.
        let yaml_name = idents.name().to_string();
        let new_conf = quote! {&conf.copy_with_new_current_boop(#yaml_name)};

        match self.0 {
            ControlType::F32 => {
                quote! {#name: murrelet_livecode::boop::BoopState::boop_init_at_time(#new_conf, t, &(target.#name as f32))}
            }
            ControlType::F32_2 => {
                quote! {#name: murrelet_livecode::boop::BoopState2::boop_init_at_time(#new_conf, t, &target.#name)}
            }
            ControlType::F32_3 => {
                quote! {#name: murrelet_livecode::boop::BoopState3::boop_init_at_time(#new_conf, t, &target.#name)}
            } //vec3(self.#name[0].boop(#new_conf, t, target.#name[0]), self.#name[1].boop(#new_conf, t, target.#name[1]), self.#name[2].boop(#new_conf, t, target.#name[2]))},
            ControlType::Color => {
                quote! {#name: murrelet_livecode::boop::BoopStateHsva::boop_init_at_time(#new_conf, t, &target.#name)}
            } //hsva(self.#name[0].boop(#new_conf, t, target.#name[0]), self.#name[1].boop(#new_conf, t, target.#name[1]), self.#name[2].boop(#new_conf, t, target.#name[2]), self.#name[3].boop(#new_conf, t, target.#name[3])).into_lin_srgba()},
            // ControlType::LinSrgbaUnclamped => quote!{#name: murrelet_livecode::livecode::ControlF32::hsva_unclamped(&self.#name)},
            ControlType::Bool => quote! {#name: target.#name}, // sorry, nothing fancy for bools yet
            ControlType::LazyNodeF32 => quote! {#name: target.#name.clone()}, // sorry, nothing fancy for bools yet
            // _ => quote!{#name: self.#name.boop(conf, t, &target.#name) as #orig_ty} // try to convert back to usize/etc
            _ => panic!("boop doesn't have {:?} yet", self.0),
        }
        // quote!{#name: self.#name.boop_init_at_time(conf, t, &target.#name)}
    }

    fn for_newtype_world(&self, _idents: StructIdents) -> TokenStream2 {
        quote! {target.0}
        // match self.0 {
        //     ControlType::F32 => quote!{self.0},
        //     //ControlType::F32_2 => quote!{#name: self.#name[0].boop(#new_conf, t, target.#name), self.#name[1].boop(conf, t, target.#name[1]))},
        //     // ControlType::F32_3 => quote!{#name: vec3(self.#name[0].boop(conf, t, target.#name[0]), self.#name[1].boop(conf, t, target.#name[1]), self.#name[2].boop(conf, t, target.#name[2]))},
        //     // ControlType::LinSrgba => quote!{#name: hsva(self.#name[0].boop(conf, t, target.#name[0]), self.#name[1].boop(conf, t, target.#name[1]), self.#name[2].boop(conf, t, target.#name[2]), self.#name[3].boop(conf, t, target.#name[3])).into_lin_srgba()},
        //     // ControlType::LinSrgbaUnclamped => quote!{#name: murrelet_livecode::livecode::ControlF32::hsva_unclamped(&self.#name)},
        //     ControlType::Bool => quote!{self.0}, // sorry, nothing fancy for bools yet
        //     // _ => quote!{#name: self.#name.boop(conf, t, &target.#name) as #orig_ty} // try to convert back to usize/etc
        //     _ => todo!("newtype world")
        // }
    }

    // this might not access the right place :)
    fn for_newtype_boop_init(&self, name: syn::Ident) -> TokenStream2 {
        let yaml_name = name.to_string();
        let new_conf = quote! {&conf.copy_with_new_current_boop(#yaml_name)};

        match self.0 {
            ControlType::F32 => {
                quote! {murrelet_livecode::boop::BoopState::boop_init_at_time(#new_conf, t, &(target.0 as f32))}
            }
            ControlType::F32_2 => {
                quote! {murrelet_livecode::boop::BoopState2::boop_init_at_time(#new_conf, t, &target.0)}
            }
            ControlType::F32_3 => {
                quote! {murrelet_livecode::boop::BoopState3::boop_init_at_time(#new_conf, t, &target.0)}
            } //vec3(self.0[0].boop(#new_conf, t, target.0[0]), self.0[1].boop(#new_conf, t, target.0[1]), self.0[2].boop(#new_conf, t, target.0[2]))},
            ControlType::Color => {
                quote! {murrelet_livecode::boop::BoopStateHsva::boop_init_at_time(#new_conf, t, &target.0)}
            } //hsva(self.0[0].boop(#new_conf, t, target.0[0]), self.0[1].boop(#new_conf, t, target.0[1]), self.0[2].boop(#new_conf, t, target.0[2]), self.0[3].boop(#new_conf, t, target.0[3])).into_lin_srgba()},
            // ControlType::LinSrgbaUnclamped => quote!{0: murrelet_livecode::livecode::ControlF32::hsva_unclamped(&self.0)},
            ControlType::Bool => quote! {target.0}, // sorry, nothing fancy for bools yet
            ControlType::LazyNodeF32 => quote! { target.0.clone() },
            // _ => quote!{0: self.0.boop(conf, t, &target.0) as #orig_ty} // try to convert back to usize/etc
            _ => panic!("boop doesn't have {:?} yet", self.0),
        }
        // quote!{#name: self.#name.boop_init_at_time(conf, t, &target.#name)}
    }
}

pub(crate) struct FieldTokensBoop {
    pub(crate) for_struct: TokenStream2,
    pub(crate) for_world: TokenStream2,
    pub(crate) for_boop_init: TokenStream2,
    pub(crate) for_boop_weird: TokenStream2,
}

impl GenFinal for FieldTokensBoop {
    fn make_struct_final(idents: ParsedFieldIdent, variants: Vec<FieldTokensBoop>) -> TokenStream2 {
        let new_ident = idents.new_ident;
        let name = idents.name;
        let vis = idents.vis;

        let for_struct = variants.iter().map(|x| x.for_struct.clone());
        let for_world = variants.iter().map(|x| x.for_world.clone());
        let for_boop_init = variants.iter().map(|x| x.for_boop_init.clone());
        let for_boop_weird = variants.iter().map(|x| x.for_boop_weird.clone());

        quote! {
            #[derive(Debug, Clone)]
            #vis struct #new_ident {
                #(#for_struct,)*
            }

            impl murrelet_livecode::boop::BoopFromWorld<#name> for #new_ident {
                fn boop(&mut self, conf: &murrelet_livecode::boop::BoopConf, t: f32, target: &#name) -> #name {
                    #name {
                        #(#for_world,)*
                    }
                }

                fn boop_init_at_time(conf: &murrelet_livecode::boop::BoopConf, t: f32, target: &#name) -> Self {
                    #new_ident {
                        #(#for_boop_init,)*
                    }
                }

                fn any_weird_states(&self) -> bool {
                    // todo, not the most efficient but not sure how to #(#something||)*
                    vec![#(#for_boop_weird,)*].iter().any(|x| *x)
                }
            }
        }
    }

    fn make_enum_final(idents: ParsedFieldIdent, variants: Vec<FieldTokensBoop>) -> TokenStream2 {
        let new_ident = idents.new_ident;
        let vis = idents.vis;
        let name = idents.name;

        let for_struct = variants.iter().map(|x| x.for_struct.clone());
        let for_world = variants.iter().map(|x| x.for_world.clone());
        let for_boop_init = variants.iter().map(|x| x.for_boop_init.clone());
        let for_boop_weird = variants.iter().map(|x| x.for_boop_weird.clone());

        quote! {
            #[derive(Debug, Clone)]
            #[allow(non_camel_case_types)]
            #vis enum #new_ident {
                #(#for_struct,)*
            }
            impl murrelet_livecode::boop::BoopFromWorld<#name> for #new_ident {
                fn boop(&mut self, conf: &murrelet_livecode::boop::BoopConf, t: f32, target: &#name) -> #name {
                    match (self, target) {
                        #(#for_world,)*
                        _ => {
                            // // the enum kind changed, so reset
                            // *self = Self::boop_init_at_time(conf, t, &target);
                            target.clone()
                        }
                    }
                }

                fn boop_init_at_time(conf: &murrelet_livecode::boop::BoopConf, t: f32, target: &#name) -> Self {
                    match target {
                        #(#for_boop_init,)*
                    }
                }

                fn any_weird_states(&self) -> bool {
                    match self {
                        #(#for_boop_weird,)*
                    }
                }
            }
        }
    }

    fn make_newtype_struct_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensBoop>,
    ) -> TokenStream2 {
        let new_ident = idents.new_ident;
        let name = idents.name;
        let vis = idents.vis;

        let for_struct = variants.iter().map(|x| x.for_struct.clone());
        let for_world = variants.iter().map(|x| x.for_world.clone());
        let for_boop_init = variants.iter().map(|x| x.for_boop_init.clone());
        let for_boop_weird = variants.iter().map(|x| x.for_boop_weird.clone());

        quote! {
            #[derive(Debug, Clone)]
            #vis struct #new_ident(#(#for_struct,)*);

            impl murrelet_livecode::boop::BoopFromWorld<#name> for #new_ident {
                fn boop(&mut self, conf: &murrelet_livecode::boop::BoopConf, t: f32, target: &#name) -> #name {
                    #name(#(#for_world,)*)
                }

                fn boop_init_at_time(conf: &murrelet_livecode::boop::BoopConf, t: f32, target: &#name) -> Self {
                    #new_ident(#(#for_boop_init,)*)
                }

                fn any_weird_states(&self) -> bool {
                    // todo, not the most efficient but not sure how to #(#something||)*
                    vec![#(#for_boop_weird,)*].iter().any(|x| *x)
                }
            }
        }
    }

    fn new_ident(name: syn::Ident) -> syn::Ident {
        update_to_boop_ident(name)
    }

    // begin parsing the different types of fields

    fn from_newtype_struct(idents: StructIdents, parent_ident: syn::Ident) -> FieldTokensBoop {
        // f32, Vec2, etc

        let ctrl = idents.control_type();

        let for_struct = {
            let t = BoopFieldType(ctrl).to_token();
            quote! {#t}
        };
        let for_world = BoopFieldType(ctrl).for_newtype_world(idents.clone());
        // send parent, not ident
        let for_boop_init = BoopFieldType(ctrl).for_newtype_boop_init(parent_ident.clone());
        let for_boop_weird = if ctrl == ControlType::Bool || ctrl == ControlType::LazyNodeF32 {
            quote! {false}
        } else {
            quote! {self.0.any_weird_states()}
        };

        FieldTokensBoop {
            for_struct,
            for_world,
            for_boop_init,
            for_boop_weird,
        }
    }

    // e.g. TileAxisLocs::V(TileAxisVs)
    fn from_unnamed_enum(idents: EnumIdents) -> FieldTokensBoop {
        let variant_ident = idents.variant_ident();
        let name = idents.enum_ident();
        let new_enum_ident = Self::new_ident(name.clone());

        let yaml_name = &format!("{}:{}", name, variant_ident);

        let unnamed = idents.data.fields.fields;

        // for struct
        if unnamed.len() != 1 {
            panic!("multiple fields not supported")
        };
        let t = unnamed.first().unwrap().clone().ty;
        let parsed_data_type = ident_from_type(&t);

        let is_lazy = parsed_data_type.main_how_to.is_lazy();

        let (for_struct, for_world, for_boop_init, for_boop_weird) = if is_lazy {
            // if it's lazy, we don't support boop on it yet, so just create a placeholder type when it's this variant
            let for_struct = quote! { #variant_ident };

            let for_world = quote! {
                (#new_enum_ident::#variant_ident, #name::#variant_ident(tar)) => {
                    #name::#variant_ident(tar.clone())
                }
            };

            let for_boop_init =
                quote! { #name::#variant_ident(targ) => #new_enum_ident::#variant_ident };

            let for_boop_weird = quote! { #new_enum_ident::#variant_ident => false };

            (for_struct, for_world, for_boop_init, for_boop_weird)
        } else {
            let new_type = update_to_boop_ident(parsed_data_type.main_type.clone());
            let for_struct = quote! { #variant_ident(#new_type) };

            let for_world = quote! {
                (#new_enum_ident::#variant_ident(s), #name::#variant_ident(tar)) => {
                    #name::#variant_ident(s.boop(&conf.copy_with_new_current_boop(#yaml_name), t, &tar))
                }
            };

            let for_boop_init = quote! { #name::#variant_ident(targ) => #new_enum_ident::#variant_ident(#new_type::boop_init_at_time(&conf.copy_with_new_current_boop(#yaml_name), t, &targ)) };

            let for_boop_weird =
                quote! { #new_enum_ident::#variant_ident(s) => s.any_weird_states() };

            (for_struct, for_world, for_boop_init, for_boop_weird)
        };

        FieldTokensBoop {
            for_struct,
            for_world,
            for_boop_init,
            for_boop_weird,
        }
    }

    // e.g. TileAxis::Diag
    fn from_unit_enum(idents: EnumIdents) -> FieldTokensBoop {
        let variant_ident = idents.variant_ident();
        let name = idents.enum_ident();
        let new_enum_ident = Self::new_ident(name.clone());

        // no-op
        let for_struct = {
            quote! { #variant_ident }
        };
        let for_world = {
            quote! { (#new_enum_ident::#variant_ident, #name::#variant_ident) => #name::#variant_ident }
        };
        let for_boop_init = {
            quote! { #name::#variant_ident => #new_enum_ident::#variant_ident }
        };
        // is never weird
        let for_boop_weird = {
            quote! { #new_enum_ident::#variant_ident => false }
        };

        FieldTokensBoop {
            for_struct,
            for_world,
            for_boop_init,
            for_boop_weird,
        }
    }

    // s: String;
    fn from_noop_struct(idents: StructIdents) -> FieldTokensBoop {
        let name = idents.name();

        let for_struct = {
            quote! {#name: ()}
        };
        let for_world = {
            quote! {#name: target.#name.clone()}
        };
        let for_boop_init = {
            quote! {#name: ()}
        };

        let for_boop_weird = {
            quote! {false}
        };

        FieldTokensBoop {
            for_struct,
            for_world,
            for_boop_init,
            for_boop_weird,
        }
    }

    // f32, Vec2, etc
    fn from_type_struct(idents: StructIdents) -> FieldTokensBoop {
        let name = idents.name();

        let ctrl = idents.control_type();

        let for_struct = {
            let t = BoopFieldType(ctrl).to_token();
            quote! {#name: #t}
        };
        let for_world = BoopFieldType(ctrl).for_world(idents.clone());
        let for_boop_init = BoopFieldType(ctrl).for_boop_init(idents.clone());
        let for_boop_weird = if ctrl == ControlType::Bool || ctrl == ControlType::LazyNodeF32 {
            quote! {false}
        } else {
            quote! {self.#name.any_weird_states()}
        };

        FieldTokensBoop {
            for_struct,
            for_world,
            for_boop_init,
            for_boop_weird,
        }
    }

    // v: Vec<f32>
    // no promises about vectors that change over time, but we try
    fn from_recurse_struct_vec(idents: StructIdents) -> FieldTokensBoop {
        let name = idents.name();
        let orig_ty = idents.orig_ty();

        let yaml_name = idents.name().to_string();

        let parsed_type_info = ident_from_type(&orig_ty);
        let how_to_control_internal = parsed_type_info.how_to_control_internal();
        let wrapper = parsed_type_info.wrapper_type();

        let for_struct = {
            let internal_type = match how_to_control_internal {
                HowToControlThis::WithType(_, c) => BoopFieldType(*c).to_token(),
                HowToControlThis::WithRecurse(_, RecursiveControlType::Struct) => {
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
            quote! {#name: #new_ty}
        };

        let for_world = {
            if how_to_control_internal.needs_to_be_evaluated() {
                match wrapper {
                    VecDepth::NotAVec => unreachable!(),
                    VecDepth::Vec => {
                        let new_conf = quote! {&conf.copy_with_new_current_boop(#yaml_name)};
                        quote! {
                            #name: {
                                let (new_x, vals) = murrelet_livecode::boop::combine_boop_vecs_for_world(#new_conf, t, &mut self.#name, &target.#name);
                                self.#name = new_x; // update the values
                                vals
                            }
                        }
                    }
                    VecDepth::VecVec => {
                        let new_conf = quote! {&conf.copy_with_new_current_boop(#yaml_name)};
                        quote! {
                            #name: {
                                let (new_x, vals) = murrelet_livecode::boop::combine_boop_vec_vecs_for_world(#new_conf, t, &mut self.#name, &target.#name);
                                self.#name = new_x; // update the values
                                vals
                            }
                        }
                    }
                }
            } else {
                quote! {#name: target.#name.clone()}
            }
        };

        let for_boop_init = {
            if how_to_control_internal.needs_to_be_evaluated() {
                let new_conf = quote! {&conf.copy_with_new_current_boop(#yaml_name)};

                match wrapper {
                    VecDepth::NotAVec => unreachable!(),
                    VecDepth::Vec => quote! {
                        #name: {
                            murrelet_livecode::boop::combine_boop_vecs_for_init(#new_conf, t, &target.#name)
                        }
                    },
                    VecDepth::VecVec => quote! {
                        #name: {
                            let mut result = Vec::with_capacity(self.#name.len());
                            for internal_row in &target.#name {
                                result.push(
                                    murrelet_livecode::boop::combine_boop_vecs_for_init(#new_conf, t, &internal_row)
                                )
                            }
                            result
                        }
                    },
                }
            } else {
                quote! {#name: target.#name.clone()}
            }
        };

        let for_boop_weird = {
            if how_to_control_internal.needs_to_be_evaluated() {
                match wrapper {
                    VecDepth::NotAVec => unreachable!(),
                    VecDepth::Vec => quote! {
                        self.#name.iter().any(|x| x.any_weird_states() )
                    },
                    VecDepth::VecVec => quote! {
                        #name: {
                            let mut any_weird_states = false;
                            for internal_row in &self.#name {
                                any_weird_states &=
                                    internal_row.iter().any(|x| x.any_weird_states() );
                            }
                            result
                        }
                    },
                }
            } else {
                quote! {false}
            }
        };

        FieldTokensBoop {
            for_struct,
            for_world,
            for_boop_init,
            for_boop_weird,
        }
    }

    fn from_newtype_recurse_struct_vec(idents: StructIdents) -> Self {
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
                        HowToControlThis::WithType(_, c) => (BoopFieldType(c).to_token(), true),
                        HowToControlThis::WithRecurse(_, RecursiveControlType::Struct) => {
                            // check if this is important!!
                            // let name = idents.config.new_ident(second_ty_ident.clone());
                            let name = Self::new_ident(second_ty_ident.clone());
                            (quote! {#name}, true)
                        }
                        HowToControlThis::WithNone(_) => (quote! {#second_ty_ident}, false),
                        e => panic!("need vec something {:?}", e),
                    }
                } else {
                    panic!("vec missing second type");
                };

                (quote! {Vec<#ref_lc_ident>}, should_o)
            };
            (quote! {#new_ty}, should_o)
        };
        let for_world = {
            if should_o {
                let new_conf = quote! { &conf.clone() };
                quote! {
                    {
                        let (new_x, vals) = murrelet_livecode::boop::combine_boop_vecs_for_world(#new_conf, t, &mut self.0, &target.0);
                        self.0 = new_x; // update the values
                        vals
                    }
                }
            } else {
                quote! {target.0.clone()}
            }
        };

        let for_boop_init = {
            if should_o {
                let new_conf = quote! { &conf.clone() };
                quote! {
                    {
                        murrelet_livecode::boop::combine_boop_vecs_for_init(#new_conf, t, &target.0)
                    }
                }
            } else {
                quote! {target.0.clone()}
            }
        };

        let for_boop_weird = {
            if should_o {
                quote! {
                    self.0.iter().any(|x| x.any_weird_states() )
                }
            } else {
                quote! {false}
            }
        };

        FieldTokensBoop {
            for_struct,
            for_world,
            for_boop_init,
            for_boop_weird,
        }
    }

    fn from_recurse_struct_struct(idents: StructIdents) -> FieldTokensBoop {
        let name = idents.name();
        let orig_ty = idents.orig_ty();
        let yaml_name = name.to_string();

        let new_ty = {
            let DataFromType { main_type, .. } = ident_from_type(&orig_ty);
            let ref_lc_ident = Self::new_ident(main_type.clone());
            quote! {#ref_lc_ident}
        };

        let for_struct = {
            quote! {#name: #new_ty}
        };
        let for_world = {
            let new_conf = quote! {&conf.copy_with_new_current_boop(#yaml_name)};
            quote! {#name: self.#name.boop(#new_conf, t, &target.#name)}
        };

        let for_boop_init = {
            let new_conf = quote! {&conf.copy_with_new_current_boop(#yaml_name)};
            quote! {#name: #new_ty::boop_init_at_time(#new_conf, t, &target.#name)}
        };

        let for_boop_weird = {
            quote! {self.#name.any_weird_states()}
        };

        FieldTokensBoop {
            for_struct,
            for_world,
            for_boop_init,
            for_boop_weird,
        }
    }

    // UnitCells<Something>
    fn from_recurse_struct_unitcell(idents: StructIdents) -> FieldTokensBoop {
        // this is similar to vec, but then we rewrap with the UnitCell info
        let name = idents.name();
        let orig_ty = idents.orig_ty();
        let yaml_name = name.to_string();

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
                            let name = update_to_boop_ident(second_ty_ident.clone());
                            quote! {Vec<#name>}
                        }

                        e => panic!("need boop something {:?}", e),
                    }
                } else {
                    panic!("boop missing second type")
                };

                quote! {#ref_lc_ident}
            };

            quote! {#name: #new_ty}
        };

        let for_world = {
            let new_conf = quote! {&conf.copy_with_new_current_boop(#yaml_name)};

            quote! {
                #name: {
                    // // split the target into nodes and sequencer
                    let (targets, details): (Vec<_>, Vec<_>) = target.#name.iter().map(|x| {(*(x.node).clone(), x.detail.clone()) }).unzip();
                    let (new_x, vals) = murrelet_livecode::boop::combine_boop_vecs_for_world(#new_conf, t, &mut self.#name, &targets);
                    self.#name = new_x; // update the values
                    vals.into_iter().zip(details.into_iter()).map(|(node, detail)| {
                        murrelet_livecode::unitcells::UnitCell::new(node, detail)
                    }).collect()
                }
            }
        };

        let for_boop_init = {
            let new_conf = quote! {&conf.copy_with_new_current_boop(#yaml_name)};

            quote! {
                #name: {
                    murrelet_livecode::boop::combine_boop_vecs_for_init(#new_conf, t, &target.#name.iter().map(|x| *(x.node).clone()).collect())
                }
            }
        };

        let for_boop_weird = {
            quote! {self.#name.iter().any(|x| x.any_weird_states())}
        };

        FieldTokensBoop {
            for_struct,
            for_world,
            for_boop_init,
            for_boop_weird,
        }
    }

    fn from_recurse_struct_lazy(idents: StructIdents) -> Self {
        Self::from_noop_struct(idents)
    }
}
