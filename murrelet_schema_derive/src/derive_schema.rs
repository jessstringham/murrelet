use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::parser::*;

pub(crate) struct FieldTokensSchema {
    pub(crate) for_make_schema: TokenStream2,
}
impl GenFinal for FieldTokensSchema {
    // Something(f32)
    fn make_newtype_struct_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensSchema>,
    ) -> TokenStream2 {
        let name = idents.name;
        let name_str = name.to_string();

        let for_make_schema = variants.iter().map(|x| x.for_make_schema.clone());

        quote! {
            impl murrelet_schema::CanMakeSchema for #name {
                fn make_schema() -> murrelet_schema::MurreletSchema {
                    murrelet_schema::MurreletSchema::new_type(#name_str.to_owned(), #(#for_make_schema,)*)
                }
            }
        }
    }

    fn make_struct_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensSchema>,
    ) -> TokenStream2 {
        let name = idents.name;
        let name_str = name.to_string();

        let for_make_schema = variants.iter().map(|x| x.for_make_schema.clone());

        quote! {
            impl murrelet_schema::CanMakeSchema for #name {
                fn make_schema() -> murrelet_schema::MurreletSchema {

                    let mut v = vec![];
                    #(#for_make_schema;)*

                    murrelet_schema::MurreletSchema::Struct(#name_str.to_owned(), v)
                }

            }
        }
    }

    fn make_enum_final(
        idents: ParsedFieldIdent,
        variants: Vec<FieldTokensSchema>,
        is_untagged: bool,
    ) -> TokenStream2 {
        let name = idents.name;
        let name_str = name.to_string();

        let for_make_schema = variants.iter().map(|x| x.for_make_schema.clone());

        quote! {
            impl murrelet_schema::CanMakeSchema for #name {
                fn make_schema() -> murrelet_schema::MurreletSchema {
                    murrelet_schema::MurreletSchema::Enum(#name_str.to_owned(), vec![#(#for_make_schema,)*], #is_untagged)
                }

            }
        }
    }

    fn from_override_enum(func: &str) -> FieldTokensSchema {
        let method: syn::Path = syn::parse_str(func).expect("Custom method is invalid path!");

        let for_make_schema = quote! {
            #method()
        };

        FieldTokensSchema { for_make_schema }
    }

    fn from_newtype_struct(idents: StructIdents, _parent_ident: syn::Ident) -> FieldTokensSchema {
        let ty = convert_vec_type(&idents.data.ty);

        let for_make_schema = quote! {
            #ty::make_schema()
        };

        FieldTokensSchema { for_make_schema }
    }

    // e.g. TileAxisLocs::V(TileAxisVs)
    fn from_unnamed_enum(idents: EnumIdents) -> FieldTokensSchema {
        let variant_ident = idents.data.ident;
        let ty = convert_vec_type(&idents.data.fields.fields.first().unwrap().ty);
        let variant_ident_str = variant_ident.to_string();

        let for_make_schema = quote! { (murrelet_schema::MurreletEnumVal::Unnamed(#variant_ident_str.to_string(), #ty::make_schema())) };

        FieldTokensSchema { for_make_schema }
    }

    // e.g. TileAxis::Diag
    fn from_unit_enum(idents: EnumIdents) -> FieldTokensSchema {
        let variant_ident = idents.data.ident;
        let variant_ident_str = variant_ident.to_string();

        let for_make_schema =
            quote! { murrelet_schema::MurreletEnumVal::Unit(#variant_ident_str.to_owned()) };

        FieldTokensSchema { for_make_schema }
    }

    // // s: String with reference
    // fn from_name(idents: StructIdents) -> FieldTokensSchema {
    //     let field_name = idents.data.ident.unwrap().to_string();

    //     let name_reference = idents
    //         .data
    //         .reference
    //         .expect("from name called without a reference!");

    //     let is_main = idents.data.is_ref_def.unwrap_or(false);

    //     let for_make_schema = quote! { v.push((#field_name.to_owned(), murrelet_schema::MurreletSchema::Val(murrelet_schema::Value::Name(#name_reference.to_owned(), #is_main)))) };

    //     FieldTokensSchema { for_make_schema }
    // }

    // skip
    fn from_noop_struct(idents: StructIdents) -> FieldTokensSchema {
        let field_name = idents.data.ident.unwrap().to_string();

        let for_make_schema =
            quote! { v.push((#field_name.to_owned(), murrelet_schema::MurreletSchema::Skip)) };

        FieldTokensSchema { for_make_schema }
    }

    // f32, Vec2, etc
    fn from_type_struct(idents: StructIdents) -> FieldTokensSchema {
        let field_name = idents.data.ident.unwrap();
        let field_name_str = field_name.to_string();
        // to call a static function, we need to
        let kind = convert_vec_type(&idents.data.ty);

        let is_flat = idents.data.flatten.unwrap_or(false);

        let for_make_schema = if is_flat {
            quote! { v.extend(#kind::make_schema().unwrap_to_struct_fields().into_iter()) }
        } else {
            quote! { v.push((#field_name_str.to_owned(), #kind::make_schema())) }
        };

        FieldTokensSchema { for_make_schema }
    }

    fn from_override_struct(idents: StructIdents, func: &str) -> FieldTokensSchema {
        let field_name = idents.data.ident.unwrap();
        let field_name_str = field_name.to_string();

        let method: syn::Path = syn::parse_str(func).expect("Custom method is invalid path!");

        let for_make_schema = quote! { v.push((#field_name_str.to_owned(), #method())) };

        FieldTokensSchema { for_make_schema }
    }
}

// we need to use turbofish to call an associated function
fn convert_vec_type(ty: &syn::Type) -> TokenStream2 {
    if let syn::Type::Path(type_path) = ty {
        if let Some(last_segment) = type_path.path.segments.last() {
            if last_segment.ident == "Vec" {
                if let syn::PathArguments::AngleBracketed(angle_bracketed) = &last_segment.arguments
                {
                    if let Some(inner_arg) = angle_bracketed.args.first() {
                        return quote! { Vec:: < #inner_arg > };
                    }
                }
            }
        }
    }

    quote! { #ty }
}
