use quote::quote;
use syn::TraitItemFn;

use crate::param_extraction::extract_params_for_call;

pub fn generate_trait_impl_methods(methods: &[&TraitItemFn]) -> Vec<proc_macro2::TokenStream> {
    methods
        .iter()
        .map(|method| {
            let sig = &method.sig;
            let method_name = &sig.ident;
            let is_async = sig.asyncness.is_some();
            let params = extract_params_for_call(method);

            let register_call = if is_async {
                quote! { self.#method_name.register_call(#params).await }
            } else {
                quote! { self.#method_name.register_call(#params) }
            };

            quote! {
                #sig {
                    #register_call
                }
            }
        })
        .collect()
}
