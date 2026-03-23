use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{ImplItem, ItemImpl, ItemTrait, TraitItem, TraitItemFn, Type};

use crate::spy_fields::{generate_spy_fields, generate_spy_initializers};
use crate::trait_impl::generate_trait_impl_methods;

pub fn generate_mock_for_trait(trait_item: ItemTrait) -> TokenStream {
    let trait_name = &trait_item.ident;
    let mock_struct_name = format_ident!("{}Mock", trait_name);
    let trait_vis = &trait_item.vis;

    let methods: Vec<_> = trait_item
        .items
        .iter()
        .filter_map(|item| {
            if let TraitItem::Fn(method) = item {
                Some(method)
            } else {
                None
            }
        })
        .collect();

    let spy_fields = generate_spy_fields(&methods);
    let spy_initializers = generate_spy_initializers(&methods);
    let trait_impl_methods = generate_trait_impl_methods(&methods);

    let has_async = methods.iter().any(|m| m.sig.asyncness.is_some());
    let async_trait_attr = if has_async {
        quote! { #[async_trait::async_trait] }
    } else {
        quote! {}
    };

    let output = quote! {
        #trait_item

        #trait_vis struct #mock_struct_name {
            #(#spy_fields)*
        }

        impl #mock_struct_name {
            pub fn new() -> Self {
                Self {
                    #(#spy_initializers)*
                }
            }
        }

        #async_trait_attr
        impl #trait_name for #mock_struct_name {
            #(#trait_impl_methods)*
        }
    };

    TokenStream::from(output)
}

pub fn generate_mock_for_impl(mut impl_item: ItemImpl) -> TokenStream {
    let self_ty = &impl_item.self_ty;
    let mock_struct_name = if let Type::Path(type_path) = &**self_ty {
        type_path.path.segments.last().unwrap().ident.clone()
    } else {
        return TokenStream::from(quote! {
            compile_error!("Cannot determine struct name from impl");
        });
    };

    let trait_path = impl_item
        .trait_
        .as_ref()
        .map(|(_, path, _)| path)
        .unwrap()
        .clone();

    let trait_methods: Vec<TraitItemFn> = impl_item
        .items
        .iter()
        .filter_map(|item| {
            if let ImplItem::Fn(method) = item {
                Some(TraitItemFn {
                    attrs: method.attrs.clone(),
                    sig: method.sig.clone(),
                    default: None,
                    semi_token: None,
                })
            } else {
                None
            }
        })
        .collect();

    let methods: Vec<_> = trait_methods.iter().collect();
    let spy_fields = generate_spy_fields(&methods);
    let spy_initializers = generate_spy_initializers(&methods);
    let trait_impl_methods = generate_trait_impl_methods(&methods);

    // Keep the original impl with #[spy_mock] removed
    let attrs_without_spy_mock: Vec<_> = impl_item
        .attrs
        .iter()
        .filter(|attr| !attr.path().is_ident("spy_mock"))
        .cloned()
        .collect();
    impl_item.attrs = attrs_without_spy_mock;

    let output = quote! {
        pub struct #mock_struct_name {
            #(#spy_fields)*
        }

        impl #mock_struct_name {
            pub fn new() -> Self {
                Self {
                    #(#spy_initializers)*
                }
            }
        }

        #impl_item

        impl #trait_path for #mock_struct_name {
            #(#trait_impl_methods)*
        }
    };

    TokenStream::from(output)
}
