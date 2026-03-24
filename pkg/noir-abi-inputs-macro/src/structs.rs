use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Ident, Type};

use crate::serde_derives;
use crate::{AbiField, AbiType, Sign};

pub(crate) fn generate_struct_def(
    struct_ident: &Ident,
    fields: &[AbiField],
    interface_module: &Type,
) -> TokenStream2 {
    let serde_derives = serde_derives();
    let field_defs = fields.iter().map(|field| {
        let name = Ident::new(&field.name, Span::call_site());
        let ty = rust_type(&field.ty, interface_module);
        quote! { pub #name: #ty, }
    });

    quote! {
        #[derive(Debug, Clone #serde_derives)]
        pub struct #struct_ident {
            #(#field_defs)*
        }
    }
}

/// Generates `FromFields` and `ToFields` trait implementations for a struct.
pub(crate) fn generate_serialization_to_fields(
    struct_ident: &Ident,
    fields: &[AbiField],
    interface_module: &Type,
) -> TokenStream2 {
    let field_count_exprs = fields.iter().map(|field| {
        let ty = rust_type(&field.ty, interface_module);
        quote! { <#ty as #interface_module::FromFields>::FIELD_COUNT }
    });

    let assignments = fields.iter().map(|field| {
        let name = Ident::new(&field.name, Span::call_site());
        let ty = rust_type(&field.ty, interface_module);
        quote! { #name: <#ty>::from_fields(iter) }
    });

    let field_serializations = fields.iter().map(|field| {
        let name = Ident::new(&field.name, Span::call_site());
        quote! { self.#name.to_fields(out) }
    });

    quote! {
        impl #interface_module::FromFields for #struct_ident {
            const FIELD_COUNT: usize = #(#field_count_exprs)+*;

            fn from_fields(iter: &mut impl Iterator<Item = #interface_module::Base>) -> Self {
                Self {
                    #(#assignments),*
                }
            }
        }

        impl #interface_module::ToFields for #struct_ident {
            fn to_fields(&self, out: &mut Vec<#interface_module::Base>) {
                #(#field_serializations);*
            }
        }
    }
}

pub(crate) fn rust_type(ty: &AbiType, interface_module: &Type) -> TokenStream2 {
    match ty {
        AbiType::Field => quote! { #interface_module::Element },
        AbiType::Integer { sign, width } => rust_type_for_integer(sign, *width),
        AbiType::Boolean => quote! { bool },
        AbiType::String { .. } => quote! { String },

        AbiType::Array { length, ty } => {
            let inner = rust_type(ty, interface_module);
            quote! { [#inner; #length] }
        }
        AbiType::Tuple { fields } => {
            let types = fields.iter().map(|f| rust_type(f, interface_module));
            quote! { (#(#types),*) }
        }

        AbiType::Struct { path, .. } => path.type_path(),
    }
}

fn rust_type_for_integer(sign: &Sign, width: u32) -> TokenStream2 {
    match (sign, width) {
        (Sign::Unsigned, 8) => quote! { u8 },
        (Sign::Signed, 8) => quote! { i8 },
        (Sign::Unsigned, 16) => quote! { u16 },
        (Sign::Signed, 16) => quote! { i16 },
        (Sign::Unsigned, 32) => quote! { u32 },
        (Sign::Signed, 32) => quote! { i32 },
        (Sign::Unsigned, 64) => quote! { u64 },
        (Sign::Signed, 64) => quote! { i64 },
        (Sign::Unsigned, 128) => quote! { u128 },
        (Sign::Signed, 128) => quote! { i128 },
        _ => panic!("Unsupported integer width {width}"),
    }
}
