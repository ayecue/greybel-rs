use duplex_protocol::{Definition, FieldsPlan, TagList, Value};
use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote::quote;

use super::names::NameProvider;

/// Generates `duplex-core` tag metadata and access trait implementations.
pub struct TagsBuilder<'a> {
    core: &'a TokenStream2,
    definition: &'a Definition,
}

impl<'a> TagsBuilder<'a> {
    pub fn new(core: &'a TokenStream2, definition: &'a Definition) -> Self {
        Self { core, definition }
    }

    /// Converts `tags` to a `&'static [Tag]` expression.
    pub fn tags(&self, tags: &TagList) -> TokenStream2 {
        let core = self.core;
        let tags = tags.all().iter().map(|tag| {
            let name = &tag.name;
            let value = self.value(self.definition.tag_value(tag));

            quote!(#core::Tag { name: #name, value: #value })
        });

        quote! {{
            const TAGS: &[#core::Tag] = &[#(#tags),*];
            TAGS
        }}
    }

    fn value(&self, value: Option<Value>) -> TokenStream2 {
        let core = self.core;

        match value {
            None | Some(Value::Null) => quote!(#core::TagValue::Flag),
            Some(Value::Bool(value)) => quote!(#core::TagValue::Bool(#value)),
            Some(Value::Integer(value)) => {
                let value = Literal::i128_unsuffixed(value);

                quote!(#core::TagValue::Integer(#value))
            }
            Some(Value::Float(value)) => {
                let value = Literal::f64_unsuffixed(value);

                quote!(#core::TagValue::Float(#value))
            }
            Some(Value::Text(value)) => quote!(#core::TagValue::Text(#value)),
            Some(Value::Char(value)) => quote!(#core::TagValue::Char(#value)),
            Some(Value::Member { name, value }) => {
                quote!(#core::TagValue::Member { name: #name, value: #value })
            }
        }
    }

    /// Converts fields and tags in `fields` to a `&'static [FieldTagContainer]` expression.
    pub fn field_tags(&self, fields: &FieldsPlan<'_>) -> TokenStream2 {
        let core = self.core;
        let fields = fields.fields().map(|field| {
            let name = &field.field.name;
            let tags = self.tags(&field.field.tags);

            quote!(#core::FieldTagContainer { name: #name, tags: #tags })
        });

        quote! {{
            const FIELDS: &[#core::FieldTagContainer] = &[#(#fields),*];
            FIELDS
        }}
    }

    /// Generates `Metadata`, `FieldAccess`, and `AsValueRef` for a message or struct with `fields`.
    pub fn structure(
        &self,
        ty: &TokenStream2,
        generics: Option<&TokenStream2>,
        tags: &TagList,
        fields: &FieldsPlan<'_>,
    ) -> TokenStream2 {
        let core = self.core;
        let tags = self.tags(tags);
        let field_tags = self.field_tags(fields);
        let names = fields.fields().map(|field| &field.field.name);
        let idents = fields
            .fields()
            .map(|field| NameProvider::field(field.field));

        quote! {
            impl #generics #core::Metadata for #ty {
                const TAGS: &'static [#core::Tag] = #tags;
                const FIELDS: &'static [#core::FieldTagContainer] = #field_tags;
            }

            impl #generics #core::FieldAccess for #ty {
                fn fields(&self) -> &'static [#core::FieldTagContainer] {
                    <Self as #core::Metadata>::FIELDS
                }

                fn field(&self, name: &str) -> ::std::option::Option<#core::ValueRef<'_>> {
                    match name {
                        #(#names => ::std::option::Option::Some(#core::AsValueRef::as_value_ref(&self.#idents)),)*
                        _ => ::std::option::Option::None,
                    }
                }
            }

            impl #generics #core::AsValueRef for #ty {
                fn as_value_ref(&self) -> #core::ValueRef<'_> {
                    #core::ValueRef::Struct(self)
                }
            }
        }
    }
}
