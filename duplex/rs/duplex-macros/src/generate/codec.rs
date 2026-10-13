use duplex_protocol::{Bit, Field, FieldPlan, FieldValue, FieldsPlan};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::Ident;

use super::names::NameProvider;
use super::rust_type::{Form, RustTypeMapper};

pub struct FieldsCodec<'a> {
    core: &'a TokenStream2,
    types: &'a RustTypeMapper<'a>,
    plan: &'a FieldsPlan<'a>,
    form: Form,
}

impl<'a> FieldsCodec<'a> {
    pub fn new(
        core: &'a TokenStream2,
        types: &'a RustTypeMapper<'a>,
        plan: &'a FieldsPlan<'a>,
        form: Form,
    ) -> Self {
        Self {
            core,
            types,
            plan,
            form,
        }
    }

    pub fn binding(field: &Field) -> Ident {
        format_ident!("field_{}", field.name)
    }

    pub fn write(&self, access: impl Fn(&Field) -> TokenStream2) -> TokenStream2 {
        let blocks = self.plan.blocks.iter().map(|block| {
            let flags = if block.flags.is_empty() {
                TokenStream2::new()
            } else {
                let count = block.flags.len();
                let sets = block
                    .fields
                    .iter()
                    .map(|field| Self::set_flags(field, &access(field.field)));

                quote! {
                    let mut flags = [0_u8; #count];
                    #(#sets)*
                    writer.raw(&flags);
                }
            };
            let values = block
                .fields
                .iter()
                .map(|field| self.write_value(field, &access(field.field)));
            let body_tokens = quote! {
                #flags
                #(#values)*
            };

            match block.since {
                0 => body_tokens,
                since => quote! {
                    if writer.minor() >= #since {
                        #body_tokens
                    }
                },
            }
        });

        quote! { #(#blocks)* }
    }

    pub fn read(&self) -> TokenStream2 {
        let blocks = self.plan.blocks.iter().map(|block| {
            let flags = if block.flags.is_empty() {
                TokenStream2::new()
            } else {
                let masks = &block.flags;

                quote! { let flags = [#(reader.flags(#masks)?),*]; }
            };
            let fields = block.fields.iter().map(|field| {
                let binding = Self::binding(field.field);
                let ty = self.types.field(field, self.form);
                let value = self.read_value(field);

                quote! { let #binding: #ty = #value; }
            });

            if block.since == 0 {
                return quote! {
                    #flags
                    #(#fields)*
                };
            }

            let since = block.since;
            let bindings: Vec<Ident> = block
                .fields
                .iter()
                .map(|field| Self::binding(field.field))
                .collect();
            let absent = block.fields.iter().map(|field| match &field.default {
                Some(default) => self.types.value(default, &field.ty),
                None => quote!(::std::option::Option::None),
            });

            quote! {
                let (#(#bindings,)*) = if reader.minor() >= #since {
                    #flags
                    #(#fields)*
                    (#(#bindings,)*)
                } else {
                    (#(#absent,)*)
                };
            }
        });

        quote! { #(#blocks)* }
    }

    /// Returns field initializers converting borrowed `Ref` bindings to the owned representation selected by `form`.
    pub fn owned_fields(
        &self,
        form: Form,
        access: impl Fn(&Field) -> TokenStream2,
    ) -> TokenStream2 {
        let fields = self.plan.fields().map(|field| {
            let ident = NameProvider::field(field.field);
            let value = access(field.field);
            let owned_expr = if field.field.optional {
                let owned_item = self.types.owned_value(&quote!(value), &field.ty, form);

                quote!(::std::option::Option::map(#value, |value| #owned_item))
            } else {
                self.types.owned_value(&value, &field.ty, form)
            };

            quote! { #ident: #owned_expr }
        });

        quote! { #(#fields),* }
    }

    pub fn construct(&self) -> TokenStream2 {
        let fields = self.plan.fields().map(|field| {
            let ident = NameProvider::field(field.field);
            let binding = Self::binding(field.field);

            quote! { #ident: #binding }
        });

        quote! { #(#fields),* }
    }

    fn flag(field: &FieldPlan<'_>) -> Option<Bit> {
        match field.value {
            FieldValue::Flag(bit) => Some(bit),
            _ => None,
        }
    }

    fn set_flags(field: &FieldPlan<'_>, access: &TokenStream2) -> TokenStream2 {
        let leaf = |value: &TokenStream2| match Self::flag(field) {
            Some(bit) => {
                let set = Self::set(bit);

                quote! { if *(#value) { #set } }
            }
            None => TokenStream2::new(),
        };
        let non_null = |value: &TokenStream2| match field.presence.non_null {
            Some(bit) => {
                let set = Self::set(bit);
                let leaf = leaf(&quote!(value));

                quote! { if let ::std::option::Option::Some(value) = #value { #set #leaf } }
            }
            None => leaf(value),
        };

        match field.presence.present {
            Some(bit) => {
                let set = Self::set(bit);
                let non_null = non_null(&quote!(value));

                quote! { if let ::std::option::Option::Some(value) = #access { #set #non_null } }
            }
            None => non_null(access),
        }
    }

    fn write_value(&self, field: &FieldPlan<'_>, access: &TokenStream2) -> TokenStream2 {
        if Self::flag(field).is_some() {
            return TokenStream2::new();
        }

        let core = self.core;
        let leaf = |value: &TokenStream2| quote! { #core::BinaryEncode::write(#value, writer)?; };
        let non_null = |value: &TokenStream2| {
            if field.presence.non_null.is_some() {
                let leaf = leaf(&quote!(value));

                quote! { if let ::std::option::Option::Some(value) = #value { #leaf } }
            } else {
                leaf(value)
            }
        };

        if field.presence.present.is_some() {
            let non_null = non_null(&quote!(value));

            quote! { if let ::std::option::Option::Some(value) = #access { #non_null } }
        } else {
            non_null(access)
        }
    }

    pub fn validate(&self, access: impl Fn(&Field) -> TokenStream2) -> TokenStream2 {
        let validations = self.plan.fields().filter_map(|field| {
            self.types
                .verify(&access(field.field), &field.ty, Some(field.position))
        });

        quote! { #(#validations)* }
    }

    fn read_value(&self, field: &FieldPlan<'_>) -> TokenStream2 {
        let leaf = match Self::flag(field) {
            Some(bit) => Self::is_set(bit),
            None => self
                .types
                .validated_read(field.ty.non_null(), Some(field.position), self.form),
        };
        let non_null = match field.presence.non_null {
            Some(bit) => Self::when(bit, &leaf),
            None => leaf,
        };

        match field.presence.present {
            Some(bit) => Self::when(bit, &non_null),
            None => non_null,
        }
    }

    fn set(bit: Bit) -> TokenStream2 {
        let Bit { byte, mask } = bit;

        quote! { flags[#byte] |= #mask; }
    }

    fn is_set(bit: Bit) -> TokenStream2 {
        let Bit { byte, mask } = bit;

        quote! { flags[#byte] & #mask != 0 }
    }

    fn when(bit: Bit, value: &TokenStream2) -> TokenStream2 {
        let is_set = Self::is_set(bit);

        quote! {
            if #is_set {
                ::std::option::Option::Some(#value)
            } else {
                ::std::option::Option::None
            }
        }
    }
}
