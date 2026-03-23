use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
#[cfg(feature = "serde")]
use quote::quote;
use serde::Deserialize;
use syn::parse::{Parse, ParseStream};
use syn::token::Comma;
use syn::{Ident, LitBool, LitStr, Result as SynResult, Type};

use input_expand::expand_noir_abi_inputs;
use qualified_core::expand_noir_abi_shared_structs;
use struct_path::StructPath;

pub(crate) mod fixtures;
pub(crate) mod input_expand;
pub(crate) mod input_value;
pub(crate) mod proof_inputs;
pub(crate) mod qualified_core;
pub(crate) mod qualified_modules;
pub(crate) mod struct_path;
pub(crate) mod structs;
pub(crate) mod to_pascal_case;

#[cfg(test)]
mod tests;

#[cfg(feature = "serde")]
fn serde_derives() -> TokenStream2 {
    quote! { , ::serde::Serialize, ::serde::Deserialize }
}

#[cfg(not(feature = "serde"))]
fn serde_derives() -> TokenStream2 {
    TokenStream2::new()
}

#[proc_macro]
pub fn noir_abi_inputs(input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(input as MacroArgs);
    match expand_noir_abi_inputs(args) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

#[proc_macro]
pub fn noir_abi_shared_structs(input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(input as SharedMacroArgs);
    match expand_noir_abi_shared_structs(args) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

struct MacroArgs {
    path: LitStr,
    module: Ident,
    oracle_hash_keccak: LitBool,
    interface_module: Type,
}

impl Parse for MacroArgs {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let path = input.parse()?;
        let _: Comma = input.parse()?;
        let module = input.parse()?;
        let _: Comma = input.parse()?;
        let oracle_hash_keccak = input.parse()?;
        let _: Comma = input.parse()?;
        let interface_module = input.parse()?;

        Ok(MacroArgs {
            path,
            module,
            oracle_hash_keccak,
            interface_module,
        })
    }
}

struct SharedMacroArgs {
    path: LitStr,
    interface_module: Type,
}

impl Parse for SharedMacroArgs {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let path = input.parse()?;
        let _: Comma = input.parse()?;
        let interface_module = input.parse()?;

        Ok(Self {
            path,
            interface_module,
        })
    }
}

#[derive(Debug, Deserialize)]
struct ProgramArtifact {
    abi: Abi,
}

#[derive(Debug, Deserialize)]
struct Abi {
    parameters: Vec<AbiParam>,
}

#[derive(Debug, Deserialize)]
struct AbiParam {
    #[serde(flatten)]
    field: AbiField,
    visibility: AbiVisibility,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum AbiVisibility {
    Public,
    Private,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum AbiType {
    Field,
    Array {
        length: usize,
        #[serde(rename = "type")]
        ty: Box<AbiType>,
    },
    Integer {
        sign: Sign,
        width: u32,
    },
    Boolean,
    Struct {
        path: StructPath,
        fields: Vec<AbiField>,
    },
    Tuple {
        fields: Vec<AbiType>,
    },
    String {
        length: u32,
    },
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Sign {
    Unsigned,
    Signed,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
struct AbiField {
    name: String,
    #[serde(rename = "type")]
    ty: AbiType,
}
