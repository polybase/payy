use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput};

use crate::attrs::find_http_error_attr;
use crate::data_structs::generate_data_structs;
use crate::fallback::build_fallback_match_arm;
use crate::from_http::build_match_arms_from_http;
use crate::to_http::build_match_arms_to_http;

pub fn derive_http_error(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    match derive_http_error_impl(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn derive_http_error_impl(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let enum_name = &input.ident;
    let variants = match &input.data {
        Data::Enum(data_enum) => &data_enum.variants,
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "HTTPErrorConversion can only be derived for enums",
            ));
        }
    };

    let http_attrs = variants
        .iter()
        .map(find_http_error_attr)
        .collect::<syn::Result<Vec<_>>>()?;

    let data_structs = generate_data_structs(variants, &http_attrs)?;
    let match_arms_to_http = build_match_arms_to_http(enum_name, variants, &http_attrs)?;
    let match_arms_from_http = build_match_arms_from_http(enum_name, variants, &http_attrs)?;
    let fallback_match_arm = build_fallback_match_arm(enum_name, variants, &http_attrs)?;

    Ok(quote! {
        #data_structs

        // Note: All data types used in tuple variants must implement Clone
        impl From<#enum_name> for HTTPError {
            fn from(err: #enum_name) -> Self {
                match err {
                    #(#match_arms_to_http)*
                }
            }
        }

        impl std::convert::TryFrom<HTTPError> for #enum_name {
            type Error = TryFromHTTPError;

            fn try_from(http_error: HTTPError) -> std::result::Result<Self, Self::Error> {
                match http_error.reason.as_str() {
                    #(#match_arms_from_http)*
                    #fallback_match_arm
                }
            }
        }

        impl std::convert::TryFrom<ErrorOutput> for #enum_name {
            type Error = TryFromHTTPError;

            fn try_from(error_output: ErrorOutput) -> std::result::Result<Self, Self::Error> {
                let http_error = HTTPError::new(
                    error_output.error.code,
                    &error_output.error.reason,
                    None,
                    error_output.error.data,
                );

                Self::try_from(http_error)
            }
        }
    })
}
