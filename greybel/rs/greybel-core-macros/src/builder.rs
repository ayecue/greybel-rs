use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput};

use crate::tagged_fields::FieldTagState;

pub struct Builder<'a> {
    input: &'a DeriveInput,
    tagged_fields: FieldTagState<'a>,
    ast: TokenStream2,
}

impl<'a> Builder<'a> {
    pub fn new(
        tagged_fields: FieldTagState<'a>,
        ast: TokenStream2,
        input: &'a DeriveInput,
    ) -> Self {
        Self {
            input,
            tagged_fields,
            ast,
        }
    }

    pub fn from_input(input: &'a DeriveInput) -> Result<Self, syn::Error> {
        let Data::Struct(data) = &input.data else {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "NodeFieldContainer can only be derived for structs",
            ));
        };
        let tagged_fields = FieldTagState::from_fields(&data.fields)?;

        Ok(Self::new(tagged_fields, quote!(::greybel_core::ast), input))
    }

    fn generate_children_collector(&self) -> TokenStream2 {
        let children = &self.tagged_fields.children;
        let ast = &self.ast;

        if children.is_empty() {
            quote!(::std::vec::Vec::new())
        } else {
            quote!({
                let mut out = ::std::vec::Vec::new();
                #( #ast::ChildField::collect(&self.#children, lists, &mut out); )*
                out
            })
        }
    }

    fn generate_block_body(&self) -> (TokenStream2, TokenStream2) {
        let tagged_fields = &self.tagged_fields;
        let ast = &self.ast;
        let ident = &self.input.ident;
        let (impl_generics, ty_generics, where_clause) = &self.input.generics.split_for_impl();

        match tagged_fields.body {
            Some(field) => (
                quote! {
                    fn as_block(&self) -> ::std::option::Option<&dyn #ast::Block> {
                        ::std::option::Option::Some(self)
                    }

                    fn as_block_mut(&mut self) -> ::std::option::Option<&mut dyn #ast::Block> {
                        ::std::option::Option::Some(self)
                    }
                },
                quote! {
                    impl #impl_generics #ast::Block for #ident #ty_generics #where_clause {
                        fn body(&self) -> #ast::NodeList {
                            self.#field
                        }

                        fn set_body(&mut self, body: #ast::NodeList) {
                            self.#field = body;
                        }
                    }
                },
            ),
            None => (TokenStream2::new(), TokenStream2::new()),
        }
    }

    fn generate_scope_body(&self) -> (TokenStream2, TokenStream2) {
        let tagged_fields = &self.tagged_fields;
        let ast = &self.ast;
        let ident = &self.input.ident;
        let (impl_generics, ty_generics, where_clause) = &self.input.generics.split_for_impl();

        match tagged_fields.scope {
            Some(field) => (
                quote! {
                    fn as_scope(&self) -> ::std::option::Option<&dyn #ast::Scope> {
                        ::std::option::Option::Some(self)
                    }

                    fn as_scope_mut(&mut self) -> ::std::option::Option<&mut dyn #ast::Scope> {
                        ::std::option::Option::Some(self)
                    }
                },
                quote! {
                    impl #impl_generics #ast::Scope for #ident #ty_generics #where_clause {
                        fn scope(&self) -> &#ast::ScopeData {
                            &self.#field
                        }

                        fn scope_mut(&mut self) -> &mut #ast::ScopeData {
                            &mut self.#field
                        }
                    }
                },
            ),
            None => (TokenStream2::new(), TokenStream2::new()),
        }
    }

    pub fn generate(&self) -> TokenStream2 {
        let ast = &self.ast;
        let ident = &self.input.ident;
        let (impl_generics, ty_generics, where_clause) = &self.input.generics.split_for_impl();
        let children = &self.tagged_fields.children;
        let collect = self.generate_children_collector();
        let (as_block, block_impl) = self.generate_block_body();
        let (as_scope, scope_impl) = self.generate_scope_body();

        quote! {
            impl #impl_generics #ast::NodeFieldContainer for #ident #ty_generics #where_clause {
                fn children(&self, lists: &#ast::NodeListStorage) -> ::std::vec::Vec<#ast::NodeId> {
                    let _ = lists;
                    #collect
                }

                fn map_children(
                    &mut self,
                    _f: &mut dyn ::std::ops::FnMut(#ast::Child) -> #ast::Child,
                ) {
                    #( #ast::ChildField::map(&mut self.#children, _f); )*
                }

                fn for_each_child(
                    &self,
                    _lists: &#ast::NodeListStorage,
                    _f: &mut impl ::std::ops::FnMut(#ast::NodeId),
                ) {
                    #( #ast::ChildField::visit(&self.#children, _lists, _f); )*
                }

                #as_block
                #as_scope
            }

            #block_impl
            #scope_impl
        }
    }
}
