use quote::quote;
use syn::TraitItemFn;

use crate::param_extraction::{extract_params_type, extract_return_type};
use crate::type_analysis::create_default_value_from_return_type;

pub fn generate_spy_fields(methods: &[&TraitItemFn]) -> Vec<proc_macro2::TokenStream> {
    methods
        .iter()
        .map(|method| {
            let method_name = &method.sig.ident;
            let params_type = extract_params_type(method);
            let return_type = extract_return_type(&method.sig.output);
            let is_async = method.sig.asyncness.is_some();

            let spy_type = if is_async {
                quote! { test_spy::AsyncSpy<#params_type, #return_type> }
            } else {
                quote! { test_spy::Spy<#params_type, #return_type> }
            };

            quote! {
                pub #method_name: #spy_type,
            }
        })
        .collect()
}

pub fn generate_spy_initializers(methods: &[&TraitItemFn]) -> Vec<proc_macro2::TokenStream> {
    methods
        .iter()
        .map(|method| {
            let method_name = &method.sig.ident;
            let is_async = method.sig.asyncness.is_some();

            // Create a default value that handles Result types
            let default_value = create_default_value_from_return_type(&method.sig.output);

            let spy_constructor = if is_async {
                quote! { test_spy::AsyncSpy::new(|_| #default_value) }
            } else {
                quote! { test_spy::Spy::new(|_| #default_value) }
            };

            quote! {
                #method_name: #spy_constructor,
            }
        })
        .collect()
}
