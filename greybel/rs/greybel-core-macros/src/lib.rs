//! Implements `#[derive(NodeFieldContainer)]` for `greybel_core` AST node structs.

mod builder;
mod tagged_fields;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{DeriveInput, parse_macro_input};

use crate::builder::Builder;

#[proc_macro_derive(NodeFieldContainer, attributes(child, body, scope))]
pub fn derive_node_fields(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    generate(&input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn generate(input: &DeriveInput) -> syn::Result<TokenStream2> {
    Ok(Builder::from_input(input)?.generate())
}
