use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::parser::*;

pub(crate) struct FieldTokensNestEdit {
    pub(crate) for_nestedit: TokenStream2,
}
impl GenFinal for FieldTokensNestEdit {
    // Something(f32)
    fn make_newtype_struct_final(
        idents: ParsedFieldIdent,
        _variants: Vec<FieldTokensNestEdit>,
    ) -> TokenStream2 {
        let name = idents.name;

        // let for_nestedit = variants.iter().map(|x| x.for_nestedit().clone());

        quote! {
            impl murrelet_livecode::nestedit::NestEditable for #name {
                fn nest_update(&self, mods: murrelet_livecode::nestedit::NestedMod) -> #name {
                    todo!("need to implement nestedit for newtypes")
                    // #name{#(#for_nestedit,)*}
                    // self.clone()
                }
            }
        }
    }

    fn make_struct_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensNestEdit>,
    ) -> TokenStream2 {
        let name = idents.name;

        let for_nestedit = variants.iter().map(|a| a.for_nestedit.clone());

        quote! {
            impl murrelet_livecode::nestedit::NestEditable for #name {
                fn nest_update(&self, mods: murrelet_livecode::nestedit::NestedMod) -> #name {
                    #name{#(#for_nestedit,)*}
                }
            }
        }
    }

    fn make_enum_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensNestEdit>,
    ) -> TokenStream2 {
        let name = idents.name;

        let for_nestedit = variants.iter().map(|a| a.for_nestedit.clone());

        quote! {
            impl murrelet_livecode::nestedit::NestEditable for #name {
                fn nest_update(&self, mods: murrelet_livecode::nestedit::NestedMod) -> #name {
                    let c = mods.get_curr();
                    let w = match (c, self) {
                        #(#for_nestedit,)*
                        _ => self.clone()
                    };
                    w.clone()
                }
            }
        }
    }

    fn new_ident(name: syn::Ident) -> syn::Ident {
        name.clone()
    }

    fn from_newtype_struct(_idents: StructIdents, parent_ident: syn::Ident) -> FieldTokensNestEdit {
        // let name = idents.control_type();

        // these will fall to todo!()
        let for_nestedit = quote! {
            #parent_ident(self.0.nest_update(mods))
        };

        FieldTokensNestEdit { for_nestedit }
    }

    // e.g. TileAxisLocs::V(TileAxisVs)
    fn from_unnamed_enum(idents: EnumIdents) -> FieldTokensNestEdit {
        let variant_ident = idents.variant_ident();
        let name = idents.enum_ident();

        let unnamed = idents.data.fields.fields;

        // for struct
        if unnamed.len() != 1 {
            panic!("multiple fields not supported")
        };

        // in this case, don't update the name, that's not supported yet...
        let for_nestedit = quote! {
            (_, #name::#variant_ident(e)) => #name::#variant_ident(e.nest_update(mods))
        };

        FieldTokensNestEdit { for_nestedit }
    }

    // e.g. TileAxis::Diag
    fn from_unit_enum(idents: EnumIdents) -> FieldTokensNestEdit {
        let variant_ident = idents.variant_ident();
        let variant_ident_str = variant_ident.to_string();
        let name = idents.enum_ident();
        // let new_enum_ident = idents.config.new_ident(name.clone());

        let for_nestedit =
            quote! { (Some(x), _) if x == #variant_ident_str => #name::#variant_ident };

        FieldTokensNestEdit { for_nestedit }
    }

    // s: String, context
    fn from_noop_struct(idents: StructIdents) -> FieldTokensNestEdit {
        let name = idents.name();
        let name_str = name.to_string();

        // okay right now i want strings to work the same
        // not sure how to make none work though..
        let for_nestedit = {
            quote! {
                #name: self.#name.nest_update(mods.next_loc(#name_str))
            }
        };

        FieldTokensNestEdit { for_nestedit }
    }

    // f32, Vec2, etc
    fn from_type_struct(idents: StructIdents) -> FieldTokensNestEdit {
        let name = idents.name();
        let yaml_name = name.to_string();

        // we'll just use the trait!
        let for_nestedit = quote! {
            #name: self.#name.nest_update(mods.next_loc(#yaml_name))
        };

        FieldTokensNestEdit { for_nestedit }
    }

    // v: Vec<f32>
    fn from_recurse_struct_vec(idents: StructIdents) -> FieldTokensNestEdit {
        let name = idents.name();

        let for_nestedit = {
            // todo, just clone for now
            quote! {
                #name: self.#name.clone()
            }
        };

        FieldTokensNestEdit { for_nestedit }
    }

    fn from_newtype_recurse_struct_vec(_idents: StructIdents) -> Self {
        let for_nestedit = {
            // todo, just clone for now
            quote! {
                self.0.clone()
            }
        };

        FieldTokensNestEdit { for_nestedit }
    }

    // this is the interesting one!
    fn from_recurse_struct_struct(idents: StructIdents) -> FieldTokensNestEdit {
        let name = idents.name();
        // let orig_ty = idents.orig_ty();
        let yaml_name = name.to_string();

        let for_nestedit = {
            quote! {
                #name: self.#name.nest_update(mods.next_loc(#yaml_name))
            }
        };

        FieldTokensNestEdit { for_nestedit }
    }

    // UnitCells<Something>
    fn from_recurse_struct_unitcell(idents: StructIdents) -> FieldTokensNestEdit {
        // this is similar to vec, but then we rewrap with the UnitCell info
        let name = idents.name();
        // let orig_ty = idents.orig_ty();
        // let yaml_name = name.to_string();

        // no-op
        let for_nestedit: TokenStream2 = {
            quote! {
                #name: self.#name.clone()
            }
        };

        FieldTokensNestEdit { for_nestedit }
    }

    fn from_recurse_struct_lazy(idents: StructIdents) -> Self {
        // Self::from_noop_struct(idents)

        let name = idents.name();

        let for_nestedit = {
            quote! {
                #name: self.#name.clone()
            }
        };

        FieldTokensNestEdit { for_nestedit }
    }
}
