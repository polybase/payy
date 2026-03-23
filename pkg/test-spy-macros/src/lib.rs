use proc_macro::TokenStream;
use syn::parse_macro_input;

mod mock_generation;
mod param_extraction;
mod spy_fields;
mod trait_impl;
mod type_analysis;

use mock_generation::{generate_mock_for_impl, generate_mock_for_trait};

#[proc_macro_attribute]
pub fn spy_mock(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as syn::Item);

    match item {
        syn::Item::Trait(trait_item) => generate_mock_for_trait(trait_item),
        syn::Item::Impl(impl_item) if impl_item.trait_.is_some() => {
            generate_mock_for_impl(impl_item)
        }
        _ => {
            let error = quote::quote! {
                compile_error!("#[spy_mock] can only be applied to traits or trait implementations");
            };
            TokenStream::from(error)
        }
    }
}
