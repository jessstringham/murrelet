use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::parser::*;

pub(crate) struct FieldTokensNestEdit {
    #[allow(dead_code)]
    kind: String,
    pub(crate) for_nestedit: TokenStream2,
    pub(crate) for_nestedit_get: TokenStream2,
    pub(crate) for_nestedit_get_newtype: Option<TokenStream2>, // matching missing the yaml
    pub(crate) for_nestedit_get_flatten: Option<TokenStream2>,
}
impl GenFinal for FieldTokensNestEdit {
    // Something(f32)
    fn make_newtype_struct_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensNestEdit>,
    ) -> TokenStream2 {
        let name = idents.name;
        let name_str = name.to_string();

        let for_nestedit_get_newtype = variants
            .iter()
            .map(|x| x.for_nestedit_get_newtype.as_ref().unwrap())
            .next();

        quote! {
            impl murrelet_livecode::nestedit::NestEditable for #name {
                fn nest_update(&self, mods: murrelet_livecode::nestedit::NestedMod) -> #name {
                    todo!("need to implement nestedit for newtypes")
                    // #name{#(#for_nestedit,)*}
                    // self.clone()
                }

                fn nest_get(&self, getter: &[&str]) -> murrelet_livecode::types::LivecodeResult<String> {
                    match getter {
                        #for_nestedit_get_newtype,
                        _ => {
                            Err(murrelet_livecode::types::LivecodeError::NestGetExtra(
                                format!("newtype struct {} didn't match: {}", #name_str, getter.join("."))))
                        }
                    }
                }
            }
        }
    }

    fn make_struct_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensNestEdit>,
    ) -> TokenStream2 {
        let name = idents.name;
        let name_str = name.to_string();

        let for_nestedit = variants.iter().map(|a| a.for_nestedit.clone());

        let for_nestedit_get = variants.iter().map(|a| a.for_nestedit_get.clone());

        // for serde flatten ones
        let flattened_nestedit_get = variants
            .iter()
            .filter_map(|a| a.for_nestedit_get_flatten.clone());

        let flatten_if_statement = if flattened_nestedit_get.clone().count() > 0 {
            quote! {
                for flattened in vec![#(#flattened_nestedit_get),*].into_iter() {
                    if let Ok(result) = flattened {
                        return Ok(result);
                    }
                }
            }
        } else {
            quote! {}
        };

        quote! {
            impl murrelet_livecode::nestedit::NestEditable for #name {
                fn nest_update(&self, mods: murrelet_livecode::nestedit::NestedMod) -> #name {
                    #name{#(#for_nestedit,)*}
                }

                fn nest_get(&self, getter: &[&str]) -> murrelet_livecode::types::LivecodeResult<String> {
                    match getter {
                        #(#for_nestedit_get,)*
                        _ => {
                            // if we have an error, go through and check the flattened ones
                            #flatten_if_statement;
                            Err(murrelet_livecode::types::LivecodeError::NestGetExtra(
                                format!("struct {} didn't match: {}", #name_str, getter.join("."))))
                        }
                    }
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
        let for_nestedit_get = variants.iter().map(|a| a.for_nestedit_get.clone());

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

                fn nest_get(&self, getter: &[&str]) -> murrelet_livecode::types::LivecodeResult<String> {

                    // first check the assumption
                    match self {
                        #(#for_nestedit_get,)*
                        _ => Err(murrelet_livecode::types::LivecodeError::NestGetExtra(format!("enum didn't match {}", getter.join("."))))
                    }


                }

            }
        }
    }

    fn new_ident(name: syn::Ident) -> syn::Ident {
        name.clone()
    }

    fn from_newtype_struct_lazy(idents: StructIdents, parent_ident: syn::Ident) -> Self {
        Self::from_newtype_struct_struct(idents, parent_ident)
    }

    fn from_newtype_struct_struct(
        idents: StructIdents,
        parent_ident: syn::Ident,
    ) -> FieldTokensNestEdit {
        Self::from_newtype_struct(idents, parent_ident)
    }

    fn from_newtype_struct(_idents: StructIdents, parent_ident: syn::Ident) -> FieldTokensNestEdit {
        // let name = idents.control_type();

        // these will fall to todo!()
        let for_nestedit = quote! {
            #parent_ident(self.0.nest_update(mods))
        };

        let for_nestedit_get = quote! {
            self.0.nest_get(&remaining)
        };

        let for_nestedit_get_flatten = quote! {
            self.0.nest_get(&remaining)
        };

        let for_nestedit_get_newtype = quote! {
            _ => self.0.nest_get(&getter)
        };

        FieldTokensNestEdit {
            kind: "newtype_struct".to_owned(),
            for_nestedit,
            for_nestedit_get,
            for_nestedit_get_newtype: Some(for_nestedit_get_newtype),
            for_nestedit_get_flatten: Some(for_nestedit_get_flatten),
        }
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

        let t = unnamed.first().unwrap().clone().ty;
        let parsed_data_type = ident_from_type(&t);

        // in this case, don't update the name, that's not supported yet...
        let for_nestedit = if parsed_data_type.main_how_to.is_lazy() {
            quote! {
                (_, #name::#variant_ident(e)) => #name::#variant_ident(e.clone())
            }
        } else {
            quote! {
                (_, #name::#variant_ident(e)) => #name::#variant_ident(e.nest_update(mods))
            }
        };

        let for_nestedit_get = quote! {
            #name::#variant_ident(e) => e.nest_get(getter)
        };

        FieldTokensNestEdit {
            kind: "unnamed_enum".to_owned(),
            for_nestedit,
            for_nestedit_get,
            for_nestedit_get_newtype: None, // shouldn't have enum in newtype!
            for_nestedit_get_flatten: None,
        }
    }

    // e.g. TileAxis::Diag
    fn from_unit_enum(idents: EnumIdents) -> FieldTokensNestEdit {
        let variant_ident = idents.variant_ident();
        let variant_ident_str = variant_ident.to_string();
        let name = idents.enum_ident();
        // let new_enum_ident = idents.config.new_ident(name.clone());

        let for_nestedit =
            quote! { (Some(x), _) if x == #variant_ident_str => #name::#variant_ident };

        let for_nestedit_get = quote! {
            #name::#variant_ident => Err(murrelet_livecode::types::LivecodeError::NestGetExtra(format!("unitcell enum")))
        };

        FieldTokensNestEdit {
            kind: "unit_enum".to_owned(),
            for_nestedit,
            for_nestedit_get,
            for_nestedit_get_newtype: None, // shouldn't have enum in newtype!
            for_nestedit_get_flatten: None,
        }
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

        let for_nestedit_get = quote! {
            [#name_str, rest @ ..] => Err(murrelet_livecode::types::LivecodeError::NestGetExtra("string".to_owned()))
        };

        let for_nestedit_get_newtype = quote! {
            _ => Err(murrelet_livecode::types::LivecodeError::NestGetExtra("string".to_owned()))
        };

        FieldTokensNestEdit {
            kind: "noop".to_owned(),
            for_nestedit,
            for_nestedit_get,
            for_nestedit_get_newtype: Some(for_nestedit_get_newtype), // shouldn't have enum in newtype!
            for_nestedit_get_flatten: None,
        }
    }

    // f32, Vec2, etc
    fn from_type_struct(idents: StructIdents) -> FieldTokensNestEdit {
        let name = idents.name();
        let yaml_name = name.to_string();

        // we'll just use the trait! (unless it's none, then we bail
        let for_nestedit = match idents.how_to_control_this() {
            HowToControlThis::WithNone(_) => quote! {
                #name: self.#name.clone()
            },
            _ => quote! {
                #name: self.#name.nest_update(mods.next_loc(#yaml_name))
            },
        };

        let for_nestedit_get = quote! {
            [#yaml_name, rest @ ..] => self.#name.nest_get(rest)
        };

        let for_nestedit_get_newtype = quote! {
            _ => self.#name.nest_get(getter)
        };

        FieldTokensNestEdit {
            kind: "type struct".to_owned(),
            for_nestedit,
            for_nestedit_get,
            for_nestedit_get_newtype: Some(for_nestedit_get_newtype),
            for_nestedit_get_flatten: None,
        }
    }

    // v: Vec<f32>
    fn from_recurse_struct_vec(idents: StructIdents) -> FieldTokensNestEdit {
        let name = idents.name();
        let yaml_name = name.to_string();

        let for_nestedit = {
            // todo, just clone for now
            quote! {
                #name: self.#name.clone()
            }
        };

        let for_nestedit_get = quote! {
            [#yaml_name, num, rest @ ..] => {
                match num.parse::<usize>() {
                    Ok(index) => self.#name[index].nest_get(rest),
                    Err(_) => Err(murrelet_livecode::types::LivecodeError::NestGetExtra(format!("couldn't parse as num {}", num)))
                }
            }
        };

        let for_nestedit_get_newtype = quote! {
            [num, rest @ ..] => {
                match num.parse::<usize>() {
                    Ok(index) => self.0[index].nest_get(rest),
                    Err(_) => Err(murrelet_livecode::types::LivecodeError::NestGetExtra(format!("couldn't parse as num {}", num)))
                }
            }
        };

        FieldTokensNestEdit {
            kind: "recurse struct vec".to_owned(),
            for_nestedit,
            for_nestedit_get,
            for_nestedit_get_newtype: Some(for_nestedit_get_newtype),
            for_nestedit_get_flatten: None,
        }
    }

    // i don't remember what this is...
    fn from_newtype_recurse_struct_vec(_idents: StructIdents) -> Self {
        let for_nestedit = {
            // todo, just clone for now
            quote! {
                self.0.clone()
            }
        };

        let for_nestedit_get = quote! {
            [rest @ ..] => { self.0.nest_get(rest) }
        };

        let for_nestedit_get_newtype = quote! {
            [num, rest @ ..] => {
                match num.parse::<usize>() {
                    Ok(index) => self.0[index].nest_get(rest),
                    Err(_) => Err(murrelet_livecode::types::LivecodeError::NestGetExtra(format!("couldn't parse as num {}", num)))
                }
            }
        };

        FieldTokensNestEdit {
            kind: "newtype recurse struct vec".to_owned(),
            for_nestedit,
            for_nestedit_get,
            for_nestedit_get_newtype: Some(for_nestedit_get_newtype),
            for_nestedit_get_flatten: None,
        }
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

        let for_nestedit_get = quote! {
            [#yaml_name, rest @ ..] => self.#name.nest_get(rest)
        };

        let for_nestedit_get_newtype = quote! {
            _ => self.#name.nest_get(rest)
        };

        let for_nestedit_get_flatten = if idents.is_serde_flatten() {
            Some(quote! {
                self.#name.nest_get(getter)
            })
        } else {
            None
        };

        FieldTokensNestEdit {
            kind: "recurse struct struct".to_owned(),
            for_nestedit,
            for_nestedit_get,
            for_nestedit_get_newtype: Some(for_nestedit_get_newtype),
            for_nestedit_get_flatten,
        }
    }

    // UnitCells<Something>
    fn from_recurse_struct_unitcell(idents: StructIdents) -> FieldTokensNestEdit {
        // this is similar to vec, but then we rewrap with the UnitCell info
        let name = idents.name();
        // let orig_ty = idents.orig_ty();
        let yaml_name = name.to_string();

        // no-op
        let for_nestedit: TokenStream2 = {
            quote! {
                #name: self.#name.clone()
            }
        };

        let for_nestedit_get = quote! {
            [#yaml_name, rest @ ..] => {
                if let Some(first) = self.#name.iter().next() {
                    first.node.nest_get(rest)
                } else {
                    Err(murrelet_livecode::types::LivecodeError::NestGetExtra(format!("unitcell is empty: {}", getter.join("."))))
                }
            }
        };

        let for_nestedit_get_newtype = quote! {
            _ => {
                if let Some(first) = self.0.iter().next() {
                    first.node.nest_get(rest)
                } else {
                    Err(murrelet_livecode::types::LivecodeError::NestGetExtra(format!("unitcell is empty: {}", getter.join("."))))
                }
            }
        };

        FieldTokensNestEdit {
            kind: "recurse struct unitcell".to_owned(),
            for_nestedit,
            for_nestedit_get,
            for_nestedit_get_newtype: Some(for_nestedit_get_newtype),
            for_nestedit_get_flatten: None,
        }
    }

    fn from_recurse_struct_lazy(idents: StructIdents) -> Self {
        // Self::from_noop_struct(idents)

        let name = idents.name();
        let yaml_name = name.to_string();

        let for_nestedit = {
            quote! {
                #name: self.#name.clone()
            }
        };

        let for_nestedit_get = quote! {
            [#yaml_name, ..] => {
                Err(murrelet_livecode::types::LivecodeError::NestGetExtra(format!("lazy not implemented yet: {}", getter.join(","))))
            }
        };

        let for_nestedit_get_newtype = quote! {
            _ => {
                Err(murrelet_livecode::types::LivecodeError::NestGetExtra(format!("lazy not implemented yet: {}", getter.join(","))))
            }
        };

        FieldTokensNestEdit {
            kind: "recurse struct lazy".to_owned(),
            for_nestedit,
            for_nestedit_get,
            for_nestedit_get_newtype: Some(for_nestedit_get_newtype),
            for_nestedit_get_flatten: None,
        }
    }
}
