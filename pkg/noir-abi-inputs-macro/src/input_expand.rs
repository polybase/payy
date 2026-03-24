use std::collections::BTreeMap;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Ident, LitStr};

use crate::fixtures::{collect_structs, parse_program, resolve_fixture_paths};
use crate::input_value::generate_struct_input_value_impl;
use crate::proof_inputs::{generate_proof_inputs_impl, generate_public_inputs_impl};
use crate::structs::generate_struct_def;
use crate::to_pascal_case::to_pascal_case;
use crate::{AbiVisibility, MacroArgs};

pub(crate) fn expand_noir_abi_inputs(args: MacroArgs) -> syn::Result<TokenStream2> {
    let module_ident = args.module;
    let interface_module = &args.interface_module;
    let path_literal = args.path;
    let fixture_paths = resolve_fixture_paths(&path_literal)?;
    let include_program_path = LitStr::new(
        &fixture_paths.program_path.to_string_lossy(),
        path_literal.span(),
    );
    let include_key_path = LitStr::new(
        &fixture_paths.key_path.to_string_lossy(),
        path_literal.span(),
    );
    let include_key_fields_path = LitStr::new(
        &fixture_paths.key_fields_path.to_string_lossy(),
        path_literal.span(),
    );
    let program = parse_program(&fixture_paths.program_path)?;

    let mut structs = BTreeMap::new();
    for param in &program.abi.parameters {
        collect_structs(&param.field.ty, &mut structs);
    }

    let mut struct_defs = Vec::new();
    let mut struct_impls = Vec::new();

    for (path, fields) in &structs {
        if path.is_qualified() {
            continue;
        }
        let struct_ident = path.ident();
        struct_defs.push(generate_struct_def(&struct_ident, fields, interface_module));
        struct_impls.push(generate_struct_input_value_impl(
            &struct_ident,
            fields,
            interface_module,
        ));
    }

    let input_ident = Ident::new(
        &format!("{}Input", to_pascal_case(&module_ident.to_string())),
        module_ident.span(),
    );

    let public_params: Vec<_> = program
        .abi
        .parameters
        .iter()
        .filter_map(|param| {
            (param.visibility == AbiVisibility::Public).then_some(param.field.clone())
        })
        .collect();
    let public_inputs_ident = Ident::new(
        &format!("{}PublicInputs", to_pascal_case(&module_ident.to_string())),
        module_ident.span(),
    );

    let all_parameters = program
        .abi
        .parameters
        .iter()
        .map(|p| p.field.clone())
        .collect::<Vec<_>>();
    let input_def = generate_struct_def(&input_ident, &all_parameters, interface_module);
    let input_impl = generate_proof_inputs_impl(
        &input_ident,
        &public_inputs_ident,
        &all_parameters,
        interface_module,
    );

    let public_inputs_def =
        generate_struct_def(&public_inputs_ident, &public_params, interface_module);
    let public_inputs_impl = generate_public_inputs_impl(
        &public_inputs_ident,
        &public_params,
        interface_module,
        args.oracle_hash_keccak,
    );

    Ok(quote! {
        pub mod #module_ident {
            use acvm::AcirField;
            use #interface_module::{Base, FromFields, ToFields};
            use lazy_static::lazy_static;
            use noirc_abi::{InputMap, input_parser::InputValue};
            use std::collections::BTreeMap;
            use super::submodules;

            pub const PROGRAM: &str = include_str!(#include_program_path);
            pub const KEY: &[u8] = include_bytes!(#include_key_path);
            pub const KEY_FIELDS: &[u8] = include_bytes!(#include_key_fields_path);

            lazy_static! {
                pub static ref PROGRAM_ARTIFACT: noirc_artifacts::program::ProgramArtifact =
                    serde_json::from_str(PROGRAM).unwrap();
                pub static ref PROGRAM_COMPILED: noirc_driver::CompiledProgram =
                    noirc_driver::CompiledProgram::from(PROGRAM_ARTIFACT.clone());
                pub static ref BYTECODE: Vec<u8> = super::super::get_bytecode_from_program(PROGRAM);
                pub static ref VERIFICATION_KEY: Vec<Base> = parse_key_fields(KEY_FIELDS);
                pub static ref VERIFICATION_KEY_HASH: Base =
                    bn254_blackbox_solver::poseidon_hash(&VERIFICATION_KEY).unwrap();
            }

            fn parse_key_fields(key_fields_json: &[u8]) -> Vec<Base> {
                let fields = serde_json::from_slice::<Vec<String>>(key_fields_json).unwrap();

                fields
                    .into_iter()
                    .map(|field| Base::from_hex(&field).unwrap())
                    .collect()
            }

            #(#struct_defs)*
            #(#struct_impls)*
            #input_def
            #input_impl
            #public_inputs_def
            #public_inputs_impl
        }
    })
}
