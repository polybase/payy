extern crate proc_macro;

mod attrs;
mod data_structs;
mod derive;
mod fallback;
mod from_http;
mod to_http;
mod util;

use proc_macro::TokenStream;

/// Derive macro for implementing `From<Error>` for HTTPError and `TryFrom<HTTPError>` for Error
#[proc_macro_derive(
    HTTPErrorConversion,
    attributes(
        bad_request,
        not_found,
        already_exists,
        failed_precondition,
        payload_too_large,
        deadline_exceeded,
        internal,
        invalid_argument,
        permission_denied,
        unauthenticated,
        aborted,
        delegate
    )
)]
pub fn derive_http_error(input: TokenStream) -> TokenStream {
    derive::derive_http_error(input)
}
