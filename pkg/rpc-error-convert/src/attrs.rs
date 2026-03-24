use quote::quote;
use syn::{Attribute, Ident, LitStr, Variant, parse::ParseStream};

#[derive(Clone)]
pub struct HttpErrorMapping {
    pub error_code: proc_macro2::TokenStream,
    pub error_code_str: String,
    pub severity: Option<proc_macro2::TokenStream>,
    pub include_data: bool,
}

#[derive(Clone)]
pub enum HttpErrorAttr {
    Mapping(HttpErrorMapping),
    Delegate,
}

pub fn find_http_error_attr(variant: &Variant) -> syn::Result<Option<HttpErrorAttr>> {
    for attr in &variant.attrs {
        let ident = match attr.path().get_ident() {
            Some(ident) => ident.to_string(),
            None => continue,
        };
        let attr_value = match ident.as_str() {
            "bad_request" => Some(parse_http_error_attr(
                attr,
                quote! { ErrorCode::BadRequest },
            )?),
            "not_found" => Some(parse_http_error_attr(attr, quote! { ErrorCode::NotFound })?),
            "already_exists" => Some(parse_http_error_attr(
                attr,
                quote! { ErrorCode::AlreadyExists },
            )?),
            "failed_precondition" => Some(parse_http_error_attr(
                attr,
                quote! { ErrorCode::FailedPrecondition },
            )?),
            "payload_too_large" => Some(parse_http_error_attr(
                attr,
                quote! { ErrorCode::PayloadTooLarge },
            )?),
            "deadline_exceeded" => Some(parse_http_error_attr(
                attr,
                quote! { ErrorCode::DeadlineExceeded },
            )?),
            "internal" => Some(parse_http_error_attr(attr, quote! { ErrorCode::Internal })?),
            "invalid_argument" => Some(parse_http_error_attr(
                attr,
                quote! { ErrorCode::InvalidArgument },
            )?),
            "permission_denied" => Some(parse_http_error_attr(
                attr,
                quote! { ErrorCode::PermissionDenied },
            )?),
            "unauthenticated" => Some(parse_http_error_attr(
                attr,
                quote! { ErrorCode::Unauthenticated },
            )?),
            "aborted" => Some(parse_http_error_attr(attr, quote! { ErrorCode::Aborted })?),
            "delegate" => Some(HttpErrorAttr::Delegate),
            _ => None,
        };

        if attr_value.is_some() {
            return Ok(attr_value);
        }
    }

    Ok(None)
}

fn parse_http_error_attr(
    attr: &Attribute,
    error_code: proc_macro2::TokenStream,
) -> syn::Result<HttpErrorAttr> {
    let (error_code_str, severity, include_data) = extract_attr_details(attr)?;

    Ok(HttpErrorAttr::Mapping(HttpErrorMapping {
        error_code,
        error_code_str,
        severity,
        include_data,
    }))
}

fn extract_attr_details(
    attr: &Attribute,
) -> syn::Result<(String, Option<proc_macro2::TokenStream>, bool)> {
    let mut reason: Option<String> = None;
    let mut severity_tokens: Option<proc_macro2::TokenStream> = None;
    let mut include_data = true;

    attr.parse_args_with(|input: ParseStream<'_>| {
        while !input.is_empty() {
            if input.peek(LitStr) && reason.is_none() {
                reason = Some(input.parse::<LitStr>()?.value());
            } else {
                let name = input.parse::<Ident>()?;
                input.parse::<syn::Token![=]>()?;
                let value = input.parse::<LitStr>()?;

                match name.to_string().as_str() {
                    "severity" => {
                        severity_tokens = match value.value().to_lowercase().as_str() {
                            "warn" => Some(quote! { ::rpc::error::Severity::Warn }),
                            "error" => None,
                            other => {
                                return Err(syn::Error::new_spanned(
                                    &value,
                                    format!("Unsupported severity level: {other}"),
                                ));
                            }
                        };
                    }
                    "data" => {
                        include_data = !matches!(
                            value.value().to_lowercase().as_str(),
                            "omit" | "none" | "skip" | "false"
                        );
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            name,
                            "Unsupported attribute argument",
                        ));
                    }
                }
            }

            if input.is_empty() {
                break;
            }

            input.parse::<syn::Token![,]>()?;
        }

        Ok(())
    })?;

    let reason = reason.ok_or_else(|| {
        syn::Error::new_spanned(attr, "Expected string literal reason in attribute")
    })?;

    Ok((reason, severity_tokens, include_data))
}
