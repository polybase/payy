use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use serde::Deserialize;
use syn::Ident;

use crate::to_pascal_case::to_pascal_case;

/// Parsed struct path from Noir ABI.
/// Converts "a::b::C" to module="a_b", name="C"
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct StructPath {
    /// Flattened module name (e.g., "a_b" from "a::b::C")
    pub(crate) module: String,
    /// Struct name (e.g., "C" from "a::b::C")
    pub(crate) name: String,
}

impl StructPath {
    /// Parse a Noir path string like "a::b::C" into module and name components.
    /// The module becomes a flattened identifier by joining non-last parts with "_".
    pub(crate) fn from_noir_path(path: &str) -> Self {
        let parts: Vec<&str> = path.split("::").collect();
        let name = parts.last().map(|s| s.to_string()).unwrap_or_default();
        let module = parts[..parts.len().saturating_sub(1)].join("_");
        Self { module, name }
    }

    /// Returns true if this is a qualified struct (has a module prefix from Noir path).
    pub(crate) fn is_qualified(&self) -> bool {
        !self.module.is_empty()
    }

    /// Generate an identifier for this struct (pascal-cased name).
    pub(crate) fn ident(&self) -> Ident {
        Ident::new(&to_pascal_case(&self.name), Span::call_site())
    }

    /// Generate a type path for referencing this struct.
    /// Expects `submodules` to be in scope in the generated module.
    pub(crate) fn type_path(&self) -> TokenStream2 {
        let struct_ident = self.ident();

        if self.module.is_empty() {
            quote! { #struct_ident }
        } else {
            let module_ident = Ident::new(&self.module, Span::call_site());
            quote! { submodules::#module_ident::#struct_ident }
        }
    }
}

impl<'de> Deserialize<'de> for StructPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(StructPath::from_noir_path(&s))
    }
}
