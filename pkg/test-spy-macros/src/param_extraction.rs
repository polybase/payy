use quote::quote;
use syn::{FnArg, Pat, ReturnType, TraitItemFn, Type};

pub fn extract_params_type(method: &TraitItemFn) -> proc_macro2::TokenStream {
    let params: Vec<_> = method
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                let ty = &pat_type.ty;
                // Handle references and mutable references
                let cleaned_ty = if let Type::Reference(type_ref) = &**ty {
                    let elem = &type_ref.elem;
                    // Check if it's a slice type like &str or &[T]
                    if let Type::Path(path) = &**elem {
                        if path.path.is_ident("str") {
                            // Convert &str to String for storage
                            quote! { String }
                        } else {
                            // For other reference types, store as owned
                            quote! { #elem }
                        }
                    } else if let Type::Slice(slice_type) = &**elem {
                        // Convert &[T] to Vec<T>
                        let inner = &slice_type.elem;
                        quote! { Vec<#inner> }
                    } else {
                        quote! { #elem }
                    }
                } else {
                    quote! { #ty }
                };
                Some(cleaned_ty)
            } else {
                None
            }
        })
        .collect();

    match params.len() {
        0 => quote! { () },
        1 => params[0].clone(),
        _ => quote! { (#(#params),*) },
    }
}

pub fn extract_params_for_call(method: &TraitItemFn) -> proc_macro2::TokenStream {
    let params: Vec<_> = method
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    let name = &pat_ident.ident;
                    let ty = &pat_type.ty;
                    // Handle references by cloning/converting to owned
                    let param_value = if let Type::Reference(type_ref) = &**ty {
                        let elem = &type_ref.elem;
                        // Check if it's a slice type like &str or &[T]
                        if let Type::Path(path) = &**elem {
                            if path.path.is_ident("str") {
                                // Convert &str to String
                                quote! { #name.to_string() }
                            } else {
                                // For other reference types, clone
                                quote! { #name.clone() }
                            }
                        } else if let Type::Slice(_) = &**elem {
                            // Convert &[T] to Vec<T>
                            quote! { #name.to_vec() }
                        } else {
                            quote! { #name.clone() }
                        }
                    } else {
                        quote! { #name }
                    };
                    Some(param_value)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    match params.len() {
        0 => quote! { () },
        1 => params[0].clone(),
        _ => quote! { (#(#params),*) },
    }
}

pub fn extract_return_type(output: &ReturnType) -> proc_macro2::TokenStream {
    match output {
        ReturnType::Default => quote! { () },
        ReturnType::Type(_, ty) => quote! { #ty },
    }
}
