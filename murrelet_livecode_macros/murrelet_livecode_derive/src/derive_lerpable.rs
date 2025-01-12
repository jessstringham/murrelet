use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::parser::*;

pub(crate) struct FieldTokensLerpable {
    pub(crate) for_lerpable: TokenStream2,
}
impl GenFinal for FieldTokensLerpable {
    // Something(f32)
    fn make_newtype_struct_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensLerpable>,
    ) -> TokenStream2 {
        let name = idents.name;

        let for_lerpable = variants.iter().map(|x| x.for_lerpable.clone());

        quote! {
            impl murrelet_livecode::lerpable::Lerpable for #name {
                fn lerpify(&self, other: &Self, pct: f32) -> Self {
                    #name(#(#for_lerpable,)*)
                }
            }
        }
    }

    fn make_struct_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensLerpable>,
    ) -> TokenStream2 {
        let name = idents.name;

        let for_lerpable = variants.iter().map(|a| a.for_lerpable.clone());

        quote! {
            impl murrelet_livecode::lerpable::Lerpable for #name {
                fn lerpify(&self, other: &Self, pct: f32) -> Self {
                    #name {
                        #(#for_lerpable,)*
                    }
                }
            }
        }
    }

    fn make_enum_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensLerpable>,
    ) -> TokenStream2 {
        let name = idents.name;

        let for_lerpable = variants.iter().map(|a| a.for_lerpable.clone());

        quote! {
            impl murrelet_livecode::lerpable::Lerpable for #name {
                fn lerpify(&self, other: &Self, pct: f32) -> Self {
                    match self {
                        #(#for_lerpable,)*
                    }
                }
            }
        }
    }

    // noop...
    fn new_ident(name: syn::Ident) -> syn::Ident {
        name.clone()
    }

    fn from_newtype_struct(_idents: StructIdents, _parent_ident: syn::Ident) -> FieldTokensLerpable {
        // let name = idents.control_type();

        // these will fall to todo!()
        let for_lerpable = quote!{
            self.0.lerpify(&other.0, pct)
        };

        FieldTokensLerpable {
            for_lerpable,
        }
    }

    // e.g. TileAxisLocs::V(TileAxisVs)
    fn from_unnamed_enum(idents: EnumIdents) -> FieldTokensLerpable {
        let variant_ident = idents.variant_ident();
        let name = idents.enum_ident();

        // let unnamed = idents.data.fields.fields;

        let for_lerpable = quote! {
            #name::#variant_ident(s) => murrelet_livecode::lerpable::step(self, other, pct)
        };

        FieldTokensLerpable {
            for_lerpable,
        }
    }

    // e.g. TileAxis::Diag
    fn from_unit_enum(idents: EnumIdents) -> FieldTokensLerpable {
        let variant_ident = idents.variant_ident();
        let name = idents.enum_ident();

        let for_lerpable: TokenStream2 = {
            quote! { #name::#variant_ident => murrelet_livecode::lerpable::step(self, other, pct) }
        };

        FieldTokensLerpable {
            for_lerpable,
        }
    }

    // s: String, context
    fn from_noop_struct(idents: StructIdents) -> FieldTokensLerpable {
        let name = idents.name();

        let for_lerpable: TokenStream2 = {
            quote! { #name: murrelet_livecode::lerpable::step(&self.#name, &other.#name, pct) }
        };

        FieldTokensLerpable {
            for_lerpable,
        }
    }

    // f32, Vec2, etc
    fn from_type_struct(idents: StructIdents) -> FieldTokensLerpable {
        let name = idents.name();

        // we'll just use the trait! (unless it's none, then we bail
        let for_lerpable = {
            quote! { #name: self.#name.lerpify(&other.#name, pct) }
        };

        FieldTokensLerpable {
            for_lerpable,
        }
    }

    // v: Vec<f32>
    fn from_recurse_struct_vec(idents: StructIdents) -> FieldTokensLerpable {
        let name = idents.name();

        let for_lerpable = {
            // todo, just clone for now
            quote! {
                #name: murrelet_livecode::lerpable::combine_vecs(&self.#name, &other.#name, pct)
            }
        };

        FieldTokensLerpable {
            for_lerpable,
        }
    }

    // Thing(Vec<Something>);
    fn from_newtype_recurse_struct_vec(_idents: StructIdents) -> Self {

        // let parsed_type_info = ident_from_type(&orig_ty);
        // let how_to_control_internal = parsed_type_info.how_to_control_internal();

        let for_lerpable = {
            // todo, just clone for now
            quote! {
                murrelet_livecode::lerpable::combine_vecs(&self.0, &other.0, pct)
            }
        };


        FieldTokensLerpable {
            for_lerpable,
        }
    }

    // this is the interesting one!
    fn from_recurse_struct_struct(idents: StructIdents) -> FieldTokensLerpable {
        let name = idents.name();
        // let orig_ty = idents.orig_ty();
        // let yaml_name = name.to_string();

        let for_lerpable = {
            quote! {
                #name: self.#name.lerpify(&other.#name, pct)
            }
        };

        FieldTokensLerpable {
            for_lerpable,
        }
    }

    // UnitCells<Something>
    fn from_recurse_struct_unitcell(idents: StructIdents) -> FieldTokensLerpable {
        // this is similar to vec, but then we rewrap with the UnitCell info
        let name = idents.name();
        // let orig_ty = idents.orig_ty();
        // let yaml_name = name.to_string();

        // no-op
        let for_lerpable: TokenStream2 = {
            quote! {
                #name: UnitCells::new(murrelet_livecode::lerpable::combine_vecs(&self.#name.items, &other.#name.items, pct))
            }
        };

        FieldTokensLerpable {
            for_lerpable,
        }
    }

    fn from_recurse_struct_lazy(idents: StructIdents) -> Self {
        // Self::from_noop_struct(idents)

        let name = idents.name();

        let for_lerpable = {
            quote! {
                #name: murrelet_livecode::lerpable::step(&self.#name, &other.#name, pct)
            }
        };

        FieldTokensLerpable {
            for_lerpable,
        }
    }
}
