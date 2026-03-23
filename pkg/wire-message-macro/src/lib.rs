use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Data, DeriveInput, parse_macro_input, spanned::Spanned};

#[proc_macro_attribute]
pub fn wire_message(
    _attr: proc_macro::TokenStream,
    tokens: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input = parse_macro_input!(tokens as DeriveInput);

    let extra_attrs = quote::quote! {
        #[derive(::borsh::BorshSerialize, ::borsh::BorshDeserialize, ::wire_message::strum_macros::EnumCount)]
    };

    let enum_try_as = if has_generics(&input) {
        quote! {}
    } else {
        quote! { #[derive(::wire_message::strum_macros::EnumTryAs)]}
    };

    let check_enum = check_enum(&input);

    quote::quote! {
        #check_enum
        #extra_attrs
        #enum_try_as
        #input
    }
    .into()
}

// EnumTryAs doesn't work with generics
fn has_generics(input: &DeriveInput) -> bool {
    !input.generics.params.is_empty()
}

fn check_enum(input: &DeriveInput) -> TokenStream {
    match &input.data {
        Data::Enum(_) => quote! {},
        Data::Struct(s) => {
            quote_spanned! { s.struct_token.span() => ::core::compile_error!("`#[wire_message]` must be used on an enum")}
        }
        Data::Union(u) => {
            quote_spanned! { u.union_token.span() => ::core::compile_error!("`#[wire_message]` must be used on an enum")}
        }
    }
}
