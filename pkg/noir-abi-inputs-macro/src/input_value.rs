use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{Ident, Type};

use crate::{AbiField, AbiType};

pub(crate) fn generate_struct_input_value_impl(
    struct_ident: &Ident,
    fields: &[AbiField],
    interface_module: &Type,
) -> TokenStream2 {
    let inserts = fields.iter().map(|field| {
        let field_name = &field.name;
        let ident = Ident::new(&field.name, Span::call_site());
        let expr = input_value_expr(quote!(value.#ident), &field.ty, interface_module);
        quote! {
            map.insert(#field_name.to_owned(), #expr);
        }
    });

    quote! {
        impl From<#struct_ident> for InputValue {
            fn from(value: #struct_ident) -> Self {
                let mut map = BTreeMap::new();
                #(#inserts)*
                InputValue::Struct(map)
            }
        }
    }
}

pub(crate) fn input_value_expr(
    expr: TokenStream2,
    ty: &AbiType,
    interface_module: &Type,
) -> TokenStream2 {
    match ty {
        AbiType::Field => {
            quote! { InputValue::Field(#interface_module::Base::from(#expr)) }
        }
        AbiType::Integer { .. } => {
            quote! { InputValue::Field(#interface_module::IntToBase::to_base(#expr)) }
        }
        AbiType::Boolean => {
            quote! { InputValue::Field(#interface_module::Base::from(#expr as u128)) }
        }
        AbiType::String { .. } => quote! { InputValue::String(#expr) },
        AbiType::Array { ty, .. } => {
            let inner = input_value_expr(quote!(value), ty, interface_module);
            quote! { InputValue::Vec(#expr.map(|value| #inner).to_vec()) }
        }
        AbiType::Struct { .. } => quote! { InputValue::from(#expr) },
        AbiType::Tuple { fields } => {
            let field_exprs = (0..)
                .map(syn::Index::from)
                .zip(fields)
                .map(|(idx, ty)| input_value_expr(quote!(#expr.#idx), ty, interface_module));
            quote! { InputValue::Vec(vec![#(#field_exprs),*]) }
        }
    }
}
