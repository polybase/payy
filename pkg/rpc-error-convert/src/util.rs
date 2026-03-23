use quote::quote;
use syn::{
    Fields, FieldsNamed, GenericArgument, PathArguments, Type, Variant, punctuated::Punctuated,
    token::Comma,
};

use crate::attrs::HttpErrorAttr;

pub fn build_http_error_call(
    error_code: &proc_macro2::TokenStream,
    error_code_str: &str,
    severity: Option<&proc_macro2::TokenStream>,
    data_tokens: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    match severity {
        Some(severity) => quote! {
            HTTPError::new_with_severity(
                #error_code,
                #error_code_str,
                Some(err.into()),
                #data_tokens,
                #severity,
            )
        },
        None => quote! {
            HTTPError::new(
                #error_code,
                #error_code_str,
                Some(err.into()),
                #data_tokens,
            )
        },
    }
}

pub fn contextful_inner_type(ty: &Type) -> Option<Type> {
    let type_path = match ty {
        Type::Path(type_path) => type_path,
        _ => return None,
    };

    let segment = type_path.path.segments.last()?;
    if segment.ident != "Contextful" {
        return None;
    }

    let args = match &segment.arguments {
        PathArguments::AngleBracketed(args) => args,
        _ => return None,
    };

    let mut inner_types = args.args.iter().filter_map(|arg| match arg {
        GenericArgument::Type(inner) => Some(inner.clone()),
        _ => None,
    });
    let inner = inner_types.next()?;
    if inner_types.next().is_some() {
        return None;
    }

    Some(inner)
}

pub fn delegate_variant_info(
    variants: &Punctuated<Variant, Comma>,
    attrs: &[Option<HttpErrorAttr>],
) -> syn::Result<Option<(syn::Ident, Type)>> {
    for (variant, attr) in variants.iter().zip(attrs.iter()) {
        if !matches!(attr, Some(HttpErrorAttr::Delegate)) {
            continue;
        }

        match &variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let field = fields.unnamed.first().ok_or_else(|| {
                    syn::Error::new_spanned(variant, "delegate attribute requires a field")
                })?;
                return Ok(Some((variant.ident.clone(), field.ty.clone())));
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    variant,
                    "delegate attribute requires a single unnamed field",
                ));
            }
        }
    }

    Ok(None)
}

pub fn named_field_idents(fields: &FieldsNamed) -> syn::Result<Vec<syn::Ident>> {
    fields
        .named
        .iter()
        .map(|field| {
            field
                .ident
                .clone()
                .ok_or_else(|| syn::Error::new_spanned(field, "expected named field"))
        })
        .collect()
}
