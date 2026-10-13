use duplex_protocol::{DocumentationRef, Field, Symbol};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::Ident;

pub struct NameProvider;

impl NameProvider {
    pub fn ident(name: &str) -> Ident {
        syn::parse_str::<Ident>(name).unwrap_or_else(|_| Ident::new_raw(name, Span::call_site()))
    }

    pub fn pascal(symbol: Symbol<'_>) -> Ident {
        Self::ident(&symbol.pascal())
    }

    pub fn snake(symbol: Symbol<'_>) -> Ident {
        Self::ident(&symbol.name().snake())
    }

    /// Returns the borrowed `Ref` type identifier for declaration `name`.
    pub fn reference(name: &str) -> Ident {
        Self::ident(&format!("{}Ref", Symbol::Declared(name).pascal()))
    }

    pub fn field(field: &Field) -> Ident {
        Self::snake(Symbol::Field(field))
    }

    pub fn docs(docs: &DocumentationRef) -> TokenStream2 {
        let lines = docs.lines().iter().map(|line| format!(" {line}"));

        quote! { #(#[doc = #lines])* }
    }
}
