use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Ident, Type};

use crate::AbiField;
use crate::input_value::generate_struct_input_value_impl;
use crate::qualified_core::QualifiedModuleGroups;
use crate::structs::{generate_serialization_to_fields, generate_struct_def};
use crate::to_pascal_case::to_pascal_case;

pub(crate) fn generate_qualified_modules(
    module_groups: &QualifiedModuleGroups,
    interface_module: &Type,
) -> TokenStream2 {
    let module_defs = module_groups
        .iter()
        .map(|(module_name, structs)| {
            let module_ident = Ident::new(module_name, Span::call_site());
            let struct_defs = structs
                .iter()
                .map(|(struct_name, fields)| {
                    generate_qualified_struct_def(struct_name, fields, interface_module)
                })
                .collect::<Vec<_>>();

            quote! {
                pub mod #module_ident {
                    use #interface_module::{Base, FromFields, ToFields};
                    use noirc_abi::input_parser::InputValue;
                    use std::collections::BTreeMap;
                    use super::super::submodules;

                    #(#struct_defs)*
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        pub mod submodules {
            #(#module_defs)*
        }
    }
}

fn generate_qualified_struct_def(
    struct_name: &str,
    fields: &[AbiField],
    interface_module: &Type,
) -> TokenStream2 {
    let struct_ident = Ident::new(&to_pascal_case(struct_name), Span::call_site());

    let definition = generate_struct_def(&struct_ident, fields, interface_module);
    let input_value_impl =
        generate_struct_input_value_impl(&struct_ident, fields, interface_module);
    let fields_impls = generate_serialization_to_fields(&struct_ident, fields, interface_module);

    quote! {
        #definition
        #input_value_impl
        #fields_impls
    }
}
