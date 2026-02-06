use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::parser::*;

pub(crate) struct FieldTokensGUI {
    pub(crate) for_make_gui: TokenStream2,
    // pub(crate) for_gui_to_livecode: TokenStream2,
}
impl GenFinal for FieldTokensGUI {
    // Something(f32)
    fn make_newtype_struct_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensGUI>,
    ) -> TokenStream2 {
        let name = idents.name;
        let name_str = name.to_string();

        let for_make_gui = variants.iter().map(|x| x.for_make_gui.clone());
        // let for_gui_to_livecode = variants.iter().map(|x| x.for_gui_to_livecode.clone());

        quote! {
            impl murrelet_gui::CanMakeGUI for #name {
                fn make_gui() -> murrelet_gui::MurreletGUISchema {
                    murrelet_gui::MurreletGUISchema::new_type(#name_str.to_owned(), #(#for_make_gui,)*)
                }

                // fn gui_to_livecode(&self, gui_val: murrelet_gui::MurreletGUISchema) -> murrelet_gui::MurreletGUISchemaResult<Self>  {
                //     if let Some(s) = gui_val.as_new_type() {
                //         Ok(#name(#(#for_gui_to_livecode,)*))
                //     } else {
                //         Err(murrelet_gui::MurreletGUISchemaErr::GUIToLivecode("newtype not in newtype"))
                //     }

                // }
            }
        }
    }

    fn make_struct_final(idents: ParsedFieldIdent, variants: Vec<FieldTokensGUI>) -> TokenStream2 {
        let name = idents.name;
        let name_str = name.to_string();

        let for_make_gui = variants.iter().map(|x| x.for_make_gui.clone());

        // let for_assign_vars = variants.iter().map(|x| x.for_assign_vars.clone());
        // let for_gui_to_livecode = variants.iter().map(|x| x.for_gui_to_livecode.clone());

        quote! {
            impl murrelet_gui::CanMakeGUI for #name {
                fn make_gui() -> murrelet_gui::MurreletGUISchema {

                    let mut v = vec![];
                    #(#for_make_gui;)*

                    murrelet_gui::MurreletGUISchema::Struct(#name_str.to_owned(), v)
                }

                // fn gui_to_livecode(&self, ux_val: murrelet_gui::MurreletGUISchema) -> murrelet_gui::MurreletGUISchemaResult<Self> {

                //     #(#for_assign_vars,)*

                //     Ok(#name(#(#for_gui_to_livecode,)*))
                // }
            }
        }
    }

    fn make_enum_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensGUI>,
        is_untagged: bool,
    ) -> TokenStream2 {
        let name = idents.name;
        let name_str = name.to_string();

        let for_make_gui = variants.iter().map(|x| x.for_make_gui.clone());
        // let for_gui_to_livecode = variants.iter().map(|x| x.for_gui_to_livecode.clone());

        quote! {
            impl murrelet_gui::CanMakeGUI for #name {
                fn make_gui() -> murrelet_gui::MurreletGUISchema {
                    murrelet_gui::MurreletGUISchema::Enum(#name_str.to_owned(), vec![#(#for_make_gui,)*], #is_untagged)
                }

                // fn gui_to_livecode(&self, gui_val: murrelet_gui::MurreletGUISchema) -> murrelet_gui::MurreletGUISchemaResult<Self>  {
                //     if let Some(enum_name_and_val) = gui_val.as_enum() {
                //         let (enum_name, enum_val) = enum_name_and_val;
                //         match gui_val {
                //             #(#for_gui_to_livecode,)*
                //         }
                //     }
                // }
            }
        }
    }

    fn from_override_enum(func: &str) -> FieldTokensGUI {
        let method: syn::Path = syn::parse_str(func).expect("Custom method is invalid path!");

        let for_make_gui = quote! {
            #method()
        };

        FieldTokensGUI { for_make_gui }
    }

    fn from_newtype_struct(idents: StructIdents, _parent_ident: syn::Ident) -> FieldTokensGUI {
        let ty = convert_vec_type(&idents.data.ty);

        let for_make_gui = quote! {
            #ty::make_gui()
        };

        FieldTokensGUI { for_make_gui }
    }

    // e.g. TileAxisLocs::V(TileAxisVs)
    fn from_unnamed_enum(idents: EnumIdents) -> FieldTokensGUI {
        let variant_ident = idents.data.ident;
        let ty = convert_vec_type(&idents.data.fields.fields.first().unwrap().ty);
        let variant_ident_str = variant_ident.to_string();

        let for_make_gui = quote! { (murrelet_gui::MurreletEnumValGUI::Unnamed(#variant_ident_str.to_string(), #ty::make_gui())) };
        // let for_gui_to_livecode = quote! { murrelet_gui::MurreletEnumValGUI::Unnamed(#variant_ident_str, enum_val) => #name::#variant_ident(enum_val.gui_to_livecode()) };

        FieldTokensGUI {
            for_make_gui,
            // for_gui_to_livecode,
            // for_assign_vars: quote!(),
        }
    }

    // e.g. TileAxis::Diag
    fn from_unit_enum(idents: EnumIdents) -> FieldTokensGUI {
        let variant_ident = idents.data.ident;
        let variant_ident_str = variant_ident.to_string();

        let for_make_gui =
            quote! { murrelet_gui::MurreletEnumValGUI::Unit(#variant_ident_str.to_owned()) };
        // let for_gui_to_livecode =
        //     quote! { murrelet_gui::Unit(#variant_ident_str) => #name::#variant_ident };

        FieldTokensGUI {
            for_make_gui,
            // for_gui_to_livecode,
            // for_assign_vars: quote!(),
        }
    }

    // s: String with reference
    fn from_name(idents: StructIdents) -> FieldTokensGUI {
        let field_name = idents.data.ident.unwrap().to_string();

        let name_reference = idents
            .data
            .reference
            .expect("from name called without a reference!");

        let is_main = idents.data.is_ref_def.unwrap_or(false);

        let for_make_gui = quote! { v.push((#field_name.to_owned(), murrelet_gui::MurreletGUISchema::Val(murrelet_gui::ValueGUI::Name(#name_reference.to_owned(), #is_main)))) };

        // let for_assign_vars
        // let for_gui_to_livecode = quote! { murrelet_gui::ValueGUIResponse::Name(name) => name };

        FieldTokensGUI {
            for_make_gui,
            // for_gui_to_livecode,
            // for_assign_vars: ,
        }
    }

    // skip
    fn from_noop_struct(idents: StructIdents) -> FieldTokensGUI {
        let field_name = idents.data.ident.unwrap().to_string();

        let for_make_gui =
            quote! { v.push((#field_name.to_owned(), murrelet_gui::MurreletGUISchema::Skip)) };
        // let for_gui_to_livecode =
        //     quote! { murrelet_gui::Unit(#variant_ident_str) => #name::#variant_ident };

        FieldTokensGUI {
            for_make_gui,
            // for_gui_to_livecode,
        }
    }

    // f32, Vec2, etc
    fn from_type_struct(idents: StructIdents) -> FieldTokensGUI {
        let field_name = idents.data.ident.unwrap();
        let field_name_str = field_name.to_string();
        // to call a static function, we need to
        let kind = convert_vec_type(&idents.data.ty);

        let is_flat = idents.data.flatten.unwrap_or(false);

        let for_make_gui = if is_flat {
            quote! { v.extend(#kind::make_gui().unwrap_to_struct_fields().into_iter()) }
        } else {
            quote! { v.push((#field_name_str.to_owned(), #kind::make_gui())) }
        };

        FieldTokensGUI { for_make_gui }
    }

    fn from_override_struct(idents: StructIdents, func: &str) -> FieldTokensGUI {
        let field_name = idents.data.ident.unwrap();
        let field_name_str = field_name.to_string();

        let method: syn::Path = syn::parse_str(func).expect("Custom method is invalid path!");

        let for_make_gui = quote! { v.push((#field_name_str.to_owned(), #method())) };

        FieldTokensGUI { for_make_gui }
    }
}

// we need to use turbofish to call an associated function
fn convert_vec_type(ty: &syn::Type) -> TokenStream2 {
    if let syn::Type::Path(type_path) = ty
        && let Some(last_segment) = type_path.path.segments.last()
        && last_segment.ident == "Vec"
        && let syn::PathArguments::AngleBracketed(angle_bracketed) = &last_segment.arguments
        && let Some(inner_arg) = angle_bracketed.args.first()
    {
        return quote! { Vec:: < #inner_arg > };
    }

    quote! { #ty }
}
