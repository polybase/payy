use quote::{format_ident, quote};
use syn::{Fields, Variant, punctuated::Punctuated, token::Comma};

use crate::attrs::{HttpErrorAttr, HttpErrorMapping};
use crate::util::named_field_idents;

pub fn build_match_arms_from_http(
    enum_name: &syn::Ident,
    variants: &Punctuated<Variant, Comma>,
    attrs: &[Option<HttpErrorAttr>],
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    let mut arms = Vec::new();

    for (variant, attr) in variants.iter().zip(attrs.iter()) {
        let mapping = match attr {
            Some(HttpErrorAttr::Mapping(mapping)) => mapping,
            _ => continue,
        };
        arms.push(build_from_http_arm(enum_name, variant, mapping)?);
    }

    Ok(arms)
}

fn build_from_http_arm(
    enum_name: &syn::Ident,
    variant: &Variant,
    mapping: &HttpErrorMapping,
) -> syn::Result<proc_macro2::TokenStream> {
    let variant_name = &variant.ident;
    let error_code_str = &mapping.error_code_str;
    let include_data = mapping.include_data;

    let arm = match &variant.fields {
        Fields::Unit => quote! {
            #error_code_str => Ok(#enum_name::#variant_name),
        },
        Fields::Unnamed(fields) => {
            let field_count = fields.unnamed.len();
            match (field_count, include_data) {
                (1, true) => quote! {
                    #error_code_str => {
                        if let Some(data) = http_error.data {
                            let data = serde_json::from_value(data)
                                .map_err(|_| TryFromHTTPError::DeserializationError)?;
                            Ok(#enum_name::#variant_name(data))
                        } else {
                            Err(TryFromHTTPError::MissingData)
                        }
                    },
                },
                (1, false) => quote! {
                    #error_code_str => Err(TryFromHTTPError::MissingData),
                },
                (_, true) => {
                    let data_struct_name = format_ident!("{}Data", variant_name);
                    let field_bindings: Vec<_> = (0..field_count)
                        .map(|i| format_ident!("field_{}", i))
                        .collect();

                    quote! {
                        #error_code_str => {
                            if let Some(data) = http_error.data {
                                let #data_struct_name(#(#field_bindings),*) = serde_json::from_value(data)
                                    .map_err(|_| TryFromHTTPError::DeserializationError)?;
                                Ok(#enum_name::#variant_name(#(#field_bindings),*))
                            } else {
                                Err(TryFromHTTPError::MissingData)
                            }
                        },
                    }
                }
                (_, false) => quote! {
                    #error_code_str => Err(TryFromHTTPError::MissingData),
                },
            }
        }
        Fields::Named(fields) => {
            if include_data {
                let data_struct_name = format_ident!("{}Data", variant_name);
                let field_names = named_field_idents(fields)?;
                quote! {
                    #error_code_str => {
                        if let Some(data) = http_error.data {
                            let #data_struct_name { #(#field_names),* } = serde_json::from_value(data)
                                .map_err(|_| TryFromHTTPError::DeserializationError)?;
                            Ok(#enum_name::#variant_name { #(#field_names),* })
                        } else {
                            Err(TryFromHTTPError::MissingData)
                        }
                    },
                }
            } else {
                quote! {
                    #error_code_str => Err(TryFromHTTPError::MissingData),
                }
            }
        }
    };

    Ok(arm)
}
