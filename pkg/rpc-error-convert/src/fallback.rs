use quote::quote;
use syn::{Variant, punctuated::Punctuated, token::Comma};

use crate::attrs::HttpErrorAttr;
use crate::util::{contextful_inner_type, delegate_variant_info};

pub fn build_fallback_match_arm(
    enum_name: &syn::Ident,
    variants: &Punctuated<Variant, Comma>,
    attrs: &[Option<HttpErrorAttr>],
) -> syn::Result<proc_macro2::TokenStream> {
    let delegate_variant = delegate_variant_info(variants, attrs)?;
    let fallback = match delegate_variant {
        Some((variant_name, inner_type)) => {
            if let Some(contextful_inner) = contextful_inner_type(&inner_type) {
                quote! {
                    _ => {
                        let inner = <#contextful_inner as std::convert::TryFrom<HTTPError>>::try_from(http_error)
                            .map_err(TryFromHTTPError::from)?;
                        Ok(#enum_name::#variant_name(contextful::Contextful::from(inner)))
                    }
                }
            } else {
                quote! {
                    _ => {
                        let inner = <#inner_type as std::convert::TryFrom<HTTPError>>::try_from(http_error)
                            .map_err(TryFromHTTPError::from)?;
                        Ok(#enum_name::#variant_name(inner))
                    }
                }
            }
        }
        None => quote! {
            reason => Err(TryFromHTTPError::UnknownReason(reason.to_string())),
        },
    };

    Ok(fallback)
}
