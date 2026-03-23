use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Ident, LitBool, Type};

use crate::AbiField;
use crate::input_value::input_value_expr;
use crate::structs::generate_serialization_to_fields;

pub(crate) fn generate_proof_inputs_impl(
    input_ident: &Ident,
    public_inputs_ident: &Ident,
    params: &[AbiField],
    interface_module: &Type,
) -> TokenStream2 {
    let inserts = params.iter().map(|param| {
        let field_name = &param.name;
        let ident = Ident::new(&param.name, Span::call_site());
        let expr = input_value_expr(quote!(me.#ident), &param.ty, interface_module);
        quote! {
            map.insert(#field_name.to_owned(), #expr);
        }
    });

    quote! {
        impl crate::circuits::ProofInputs for #input_ident {
            const PROGRAM: &'static str = PROGRAM;
            const KEY: &'static [u8] = KEY;
            fn bytecode(&self) -> &[u8] { &*BYTECODE }
            fn compiled_program(&self) -> &noirc_driver::CompiledProgram { &*PROGRAM_COMPILED }
            type PublicInputs = #public_inputs_ident;

            fn input_map(&self) -> InputMap {
                let me = self.clone();
                let mut map = InputMap::new();
                #(#inserts)*
                map
            }
        }
    }
}

pub(crate) fn generate_public_inputs_impl(
    public_ident: &Ident,
    params: &[AbiField],
    interface_module: &Type,
    oracle_hash_keccak: LitBool,
) -> TokenStream2 {
    let fields_impls = generate_serialization_to_fields(public_ident, params, interface_module);

    quote! {
        #fields_impls

        impl #interface_module::PublicInputs for #public_ident {
            const KEY: &'static [u8] = KEY;
            const ORACLE_HASH_KECCAK: bool = #oracle_hash_keccak;
        }
    }
}
