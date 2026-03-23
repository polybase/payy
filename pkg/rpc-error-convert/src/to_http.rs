use quote::{format_ident, quote};
use syn::{Fields, FieldsNamed, FieldsUnnamed, Variant, punctuated::Punctuated, token::Comma};

use crate::attrs::{HttpErrorAttr, HttpErrorMapping};
use crate::util::{build_http_error_call, named_field_idents};

pub fn build_match_arms_to_http(
    enum_name: &syn::Ident,
    variants: &Punctuated<Variant, Comma>,
    attrs: &[Option<HttpErrorAttr>],
) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    variants
        .iter()
        .zip(attrs.iter())
        .map(|(variant, attr)| build_match_arm_to_http(enum_name, variant, attr))
        .collect()
}

fn build_match_arm_to_http(
    enum_name: &syn::Ident,
    variant: &Variant,
    attr: &Option<HttpErrorAttr>,
) -> syn::Result<proc_macro2::TokenStream> {
    let variant_name = &variant.ident;

    match attr {
        Some(HttpErrorAttr::Mapping(mapping)) => {
            build_mapping_arm_to_http(enum_name, variant_name, &variant.fields, mapping)
        }
        Some(HttpErrorAttr::Delegate) => build_delegate_arm_to_http(enum_name, variant),
        None => Ok(build_internal_arm_to_http(
            enum_name,
            variant_name,
            &variant.fields,
        )),
    }
}

fn build_mapping_arm_to_http(
    enum_name: &syn::Ident,
    variant_name: &syn::Ident,
    fields: &Fields,
    mapping: &HttpErrorMapping,
) -> syn::Result<proc_macro2::TokenStream> {
    match fields {
        Fields::Unit => Ok(build_unit_mapping_arm(enum_name, variant_name, mapping)),
        Fields::Unnamed(fields) => Ok(build_unnamed_mapping_arm(
            enum_name,
            variant_name,
            fields,
            mapping,
        )),
        Fields::Named(fields) => build_named_mapping_arm(enum_name, variant_name, fields, mapping),
    }
}

fn build_unit_mapping_arm(
    enum_name: &syn::Ident,
    variant_name: &syn::Ident,
    mapping: &HttpErrorMapping,
) -> proc_macro2::TokenStream {
    let http_error = build_http_error_for_mapping(mapping, quote! { None::<()> });
    quote! {
        #enum_name::#variant_name => #http_error,
    }
}

fn build_unnamed_mapping_arm(
    enum_name: &syn::Ident,
    variant_name: &syn::Ident,
    fields: &FieldsUnnamed,
    mapping: &HttpErrorMapping,
) -> proc_macro2::TokenStream {
    let field_count = fields.unnamed.len();
    match (field_count, mapping.include_data) {
        (1, true) => {
            let http_error = build_http_error_for_mapping(mapping, quote! { Some(data_clone) });
            quote! {
                #enum_name::#variant_name(ref data) => {
                    let data_clone = data.clone();
                    #http_error
                },
            }
        }
        (1, false) => {
            let http_error = build_http_error_for_mapping(mapping, quote! { None::<()> });
            quote! {
                #enum_name::#variant_name(_) => #http_error,
            }
        }
        (_, true) => {
            let data_struct_name = format_ident!("{}Data", variant_name);
            let field_names: Vec<_> = (0..field_count)
                .map(|i| format_ident!("field_{}", i))
                .collect();
            let http_error = build_http_error_for_mapping(mapping, quote! { Some(data) });
            quote! {
                #enum_name::#variant_name(#(ref #field_names),*) => {
                    let data = #data_struct_name(#(#field_names.clone()),*);
                    #http_error
                },
            }
        }
        (_, false) => {
            let http_error = build_http_error_for_mapping(mapping, quote! { None::<()> });
            quote! {
                #enum_name::#variant_name(..) => #http_error,
            }
        }
    }
}

fn build_named_mapping_arm(
    enum_name: &syn::Ident,
    variant_name: &syn::Ident,
    fields: &FieldsNamed,
    mapping: &HttpErrorMapping,
) -> syn::Result<proc_macro2::TokenStream> {
    if mapping.include_data {
        let data_struct_name = format_ident!("{}Data", variant_name);
        let field_names = named_field_idents(fields)?;
        let http_error = build_http_error_for_mapping(mapping, quote! { Some(data) });
        Ok(quote! {
            #enum_name::#variant_name { #(ref #field_names),* } => {
                let data = #data_struct_name {
                    #(#field_names: #field_names.clone()),*
                };
                #http_error
            },
        })
    } else {
        let http_error = build_http_error_for_mapping(mapping, quote! { None::<()> });
        Ok(quote! {
            #enum_name::#variant_name { .. } => #http_error,
        })
    }
}

fn build_http_error_for_mapping(
    mapping: &HttpErrorMapping,
    data_tokens: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    build_http_error_call(
        &mapping.error_code,
        &mapping.error_code_str,
        mapping.severity.as_ref(),
        data_tokens,
    )
}

fn build_delegate_arm_to_http(
    enum_name: &syn::Ident,
    variant: &Variant,
) -> syn::Result<proc_macro2::TokenStream> {
    let variant_name = &variant.ident;
    match &variant.fields {
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => Ok(quote! {
            #enum_name::#variant_name(inner) => inner.into(),
        }),
        _ => Err(syn::Error::new_spanned(
            variant,
            "delegate attribute requires a single unnamed field",
        )),
    }
}

fn build_internal_arm_to_http(
    enum_name: &syn::Ident,
    variant_name: &syn::Ident,
    fields: &Fields,
) -> proc_macro2::TokenStream {
    match fields {
        Fields::Unit => quote! {
            #enum_name::#variant_name => HTTPError::internal(err.into()),
        },
        Fields::Unnamed(_) => quote! {
            #enum_name::#variant_name(..) => HTTPError::internal(err.into()),
        },
        Fields::Named(_) => quote! {
            #enum_name::#variant_name { .. } => HTTPError::internal(err.into()),
        },
    }
}
