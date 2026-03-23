use quote::quote;
use syn::{GenericArgument, PathArguments, ReturnType, Type};

pub fn create_default_value_from_return_type(return_type: &ReturnType) -> proc_macro2::TokenStream {
    match return_type {
        ReturnType::Default => quote! { () },
        ReturnType::Type(_, ty) => create_default_value_from_type(ty),
    }
}

pub fn create_default_value_from_type(ty: &Type) -> proc_macro2::TokenStream {
    if is_result_type(ty) {
        // For Result types, return Ok with the inner type's default
        quote! { Ok(Default::default()) }
    } else if is_arc_dyn_type(ty) {
        // For Arc<dyn Trait> (or similar), default can't be derived; force tests to set via return_next
        quote! { panic!("test-spy: default not set for this return type; use return_next() in your test") }
    } else {
        // For non-Result types, use the Default trait
        quote! { <#ty>::default() }
    }
}

pub fn is_result_type(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            // Check if the last segment of the path is "Result"
            if let Some(last_segment) = type_path.path.segments.last()
                && last_segment.ident == "Result"
            {
                return true;
            }

            // Check for fully qualified Result types like std::result::Result
            for segment in &type_path.path.segments {
                if segment.ident == "Result" {
                    // Check if this appears to be std::result::Result or similar
                    let path_str = type_path
                        .path
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");
                    if path_str.contains("result::Result") || path_str.contains("std::Result") {
                        return true;
                    }
                }
            }

            false
        }
        // Handle other type variants if needed
        _ => false,
    }
}

fn is_arc_dyn_type(ty: &Type) -> bool {
    match ty {
        Type::Path(type_path) => {
            if let Some(last) = type_path.path.segments.last()
                && last.ident == "Arc"
                && let PathArguments::AngleBracketed(ref ab) = last.arguments
                && let Some(GenericArgument::Type(inner_ty)) = ab.args.first()
            {
                return matches!(inner_ty, Type::TraitObject(_));
            }
            false
        }
        _ => false,
    }
}
