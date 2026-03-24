use quote::{format_ident, quote};
use syn::{Fields, Variant, punctuated::Punctuated, token::Comma};

use crate::attrs::HttpErrorAttr;

pub fn generate_data_structs(
    variants: &Punctuated<Variant, Comma>,
    attrs: &[Option<HttpErrorAttr>],
) -> syn::Result<proc_macro2::TokenStream> {
    let structs = variants
        .iter()
        .zip(attrs.iter())
        .filter_map(|(variant, attr)| {
            let include_data = match attr {
                Some(HttpErrorAttr::Mapping(mapping)) => mapping.include_data,
                _ => false,
            };
            if !include_data {
                return None;
            }

            let variant_name = &variant.ident;
            let data_struct_name = format_ident!("{}Data", variant_name);

            let struct_tokens = match &variant.fields {
                Fields::Named(fields) => {
                    let field_definitions: syn::Result<Vec<_>> = fields
                        .named
                        .iter()
                        .map(|f| {
                            let field_name = f.ident.as_ref().ok_or_else(|| {
                                syn::Error::new_spanned(f, "expected named field")
                            })?;
                            let field_type = &f.ty;
                            let doc_attrs: Vec<_> = f
                                .attrs
                                .iter()
                                .filter(|attr| attr.path().is_ident("doc"))
                                .collect();

                            Ok(quote! {
                                #(#doc_attrs)*
                                pub #field_name: #field_type
                            })
                        })
                        .collect();

                    let field_definitions = match field_definitions {
                        Ok(fields) => fields,
                        Err(err) => return Some(Err(err)),
                    };

                    let struct_doc = format!("Data structure for {variant_name} error variant");

                    Ok(quote! {
                        #[doc = #struct_doc]
                        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
                        pub struct #data_struct_name {
                            #(#field_definitions),*
                        }
                    })
                }
                Fields::Unnamed(fields) if fields.unnamed.len() > 1 => {
                    let field_types: Vec<_> = fields.unnamed.iter().map(|f| &f.ty).collect();
                    let struct_doc =
                        format!("Data structure for {variant_name} error variant (tuple fields)");

                    Ok(quote! {
                        #[doc = #struct_doc]
                        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
                        pub struct #data_struct_name(#(pub #field_types),*);
                    })
                }
                _ => Ok(proc_macro2::TokenStream::new()),
            };

            Some(struct_tokens)
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        #(#structs)*
    })
}
