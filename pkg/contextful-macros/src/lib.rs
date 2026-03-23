extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Path, Token, parse_macro_input, punctuated::Punctuated};

#[proc_macro_derive(FromContextful, attributes(contextful))]
pub fn derive_from_contextful(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let mut map_fns = Vec::new();

    // If the source error is already this enum, discard the context and return it
    // directly to avoid nested Internal(InternalError(...)) wrappers.
    // it works like a map function fn(Contextful<#name>) -> #name
    map_fns.push(syn::parse_quote! { ::contextful::Contextful::<#name>::into_source });

    for attr in &input.attrs {
        if !attr.path().is_ident("contextful") {
            continue;
        }

        let paths = match attr.parse_args_with(Punctuated::<Path, Token![,]>::parse_terminated) {
            Ok(paths) => paths,
            Err(err) => return err.to_compile_error().into(),
        };

        for path in paths {
            map_fns.push(path);
        }
    }

    let checks = map_fns.iter().map(|fn_path| {
        quote! {
            // SAFETY: We just constructed the InternalError from a Contextful<E> above,
            // so there is exactly one strong reference to the Arc.
            let err = match unsafe { err.downcast() } {
                Ok(casted) => return #fn_path(casted).into(),
                Err(err) => err,
            };
        }
    });

    let expanded = quote! {
        impl<E> From<::contextful::Contextful<E>> for #name
        where
            E: std::error::Error + Send + Sync + 'static,
        {
            fn from(error: ::contextful::Contextful<E>) -> Self {
                fn map_contextful_error(err: ::contextful::InternalError) -> #name {
                    #(#checks)*

                    #name::Internal(err)
                }

                map_contextful_error(::contextful::InternalError::from(error))
            }
        }
    };

    expanded.into()
}
