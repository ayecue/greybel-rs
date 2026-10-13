mod analysis;
mod validations;

use std::cell::RefCell;
use std::collections::HashSet;

use duplex_protocol::{Encoding, FieldPlan, NumberKind, Plan, Value};
use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::Path;

use super::names::NameProvider;

/// Rust field value representation.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// Owned value.
    Value,
    /// Message-body value using `Cow` for strings and lists.
    Reference,
    /// Input-borrowed value for notification handlers.
    Ref,
}

pub struct RustTypeMapper<'a> {
    core: &'a TokenStream2,
    plan: &'a Plan<'a>,
    types: Option<&'a Path>,
    path_names: RefCell<HashSet<&'a str>>,
}

impl<'a> RustTypeMapper<'a> {
    pub fn new(core: &'a TokenStream2, plan: &'a Plan<'a>, types: Option<&'a Path>) -> Self {
        Self {
            core,
            plan,
            types,
            path_names: RefCell::default(),
        }
    }

    pub fn definition(&self) -> &'a duplex_protocol::Definition {
        self.plan.definition
    }

    pub fn declared_type(&self, name: &str) -> TokenStream2 {
        let ident = NameProvider::ident(name);

        match self.types {
            Some(types) => quote!(#types::#ident),
            None => quote!(#ident),
        }
    }

    pub fn declared_ref(&self, name: &str) -> TokenStream2 {
        let ident = NameProvider::reference(name);

        match self.types {
            Some(types) => quote!(#types::#ident<'a>),
            None => quote!(#ident<'a>),
        }
    }

    pub fn owned_type(&self, ty: &Encoding<'_>) -> TokenStream2 {
        match ty {
            Encoding::Bool => quote!(bool),
            Encoding::Char => quote!(char),
            Encoding::String(_) => quote!(::std::string::String),
            Encoding::Bytes(_) => {
                let core = self.core;

                quote!(#core::Bytes)
            }
            Encoding::Number(NumberKind::Fixed32, _) => {
                let core = self.core;

                quote!(#core::Fixed32)
            }
            Encoding::Number(NumberKind::Fixed64, _) => {
                let core = self.core;

                quote!(#core::Fixed64)
            }
            Encoding::Number(kind, _) => {
                let ident = format_ident!("{}", kind.keyword());

                quote!(#ident)
            }
            Encoding::List(item, _) => {
                let item = self.owned_type(item);

                quote!(::std::vec::Vec<#item>)
            }
            Encoding::Record(value, _) => {
                let value = self.owned_type(value);

                quote!(::std::collections::BTreeMap<::std::string::String, #value>)
            }
            Encoding::Nullable(inner) => {
                let inner = self.owned_type(inner);

                quote!(::std::option::Option<#inner>)
            }
            Encoding::Struct(name) | Encoding::Union(name) | Encoding::Enumeration(name) => {
                self.declared_type(name)
            }
        }
    }

    fn cow_type(&self, ty: &Encoding<'_>) -> TokenStream2 {
        match ty {
            Encoding::String(_) => quote!(::std::borrow::Cow<'a, str>),
            Encoding::List(item, _) => {
                let item = self.owned_type(item);

                quote!(::std::borrow::Cow<'a, [#item]>)
            }
            Encoding::Nullable(inner) => {
                let inner = self.cow_type(inner);

                quote!(::std::option::Option<#inner>)
            }
            _ => self.owned_type(ty),
        }
    }

    /// Returns the Rust type for an input-borrowing decoded value.
    pub fn reference(&self, ty: &Encoding<'_>) -> TokenStream2 {
        match ty {
            Encoding::String(_) => quote!(&'a str),
            Encoding::Bytes(_) => quote!(&'a [u8]),
            Encoding::List(item, _) => {
                let item = self.reference(item);

                quote!(::std::vec::Vec<#item>)
            }
            Encoding::Record(value, _) => {
                let value = self.reference(value);

                quote!(::std::collections::BTreeMap<&'a str, #value>)
            }
            Encoding::Nullable(inner) => {
                let inner = self.reference(inner);

                quote!(::std::option::Option<#inner>)
            }
            Encoding::Struct(name) | Encoding::Union(name) if !self.is_plain(ty) => {
                self.declared_ref(name)
            }
            _ => self.owned_type(ty),
        }
    }

    /// Converts `value` from borrowed `ty` representation to owned representation. `Form::Reference` wraps strings and
    /// lists in `Cow`.
    pub fn owned_value(&self, value: &TokenStream2, ty: &Encoding<'_>, form: Form) -> TokenStream2 {
        match ty {
            Encoding::String(_) if form == Form::Reference => {
                quote!(::std::borrow::Cow::Owned(::std::string::String::from(#value)))
            }
            Encoding::String(_) => quote!(::std::string::String::from(#value)),
            Encoding::Bytes(_) => {
                let core = self.core;

                quote!(#core::Bytes(::std::vec::Vec::from(#value)))
            }
            Encoding::List(item, _) => {
                let list = if self.is_plain(item) {
                    value.clone()
                } else {
                    let item = self.owned_value(&quote!(item), item, Form::Value);

                    quote! {
                        ::std::iter::Iterator::collect(
                            ::std::iter::Iterator::map(::std::iter::IntoIterator::into_iter(#value), |item| #item),
                        )
                    }
                };

                if form == Form::Reference {
                    quote!(::std::borrow::Cow::Owned(#list))
                } else {
                    list
                }
            }
            Encoding::Record(entry, _) => {
                let entry = self.owned_value(&quote!(value), entry, Form::Value);

                quote! {
                    ::std::iter::Iterator::collect(
                        ::std::iter::Iterator::map(::std::iter::IntoIterator::into_iter(#value), |(key, value)| {
                            (::std::string::String::from(key), #entry)
                        }),
                    )
                }
            }
            Encoding::Nullable(inner) => {
                let inner = self.owned_value(&quote!(value), inner, form);

                quote!(::std::option::Option::map(#value, |value| #inner))
            }
            Encoding::Struct(_) | Encoding::Union(_) if !self.is_plain(ty) => {
                quote!((#value).into_owned())
            }
            _ => value.clone(),
        }
    }

    pub fn field(&self, field: &FieldPlan<'_>, form: Form) -> TokenStream2 {
        let ty = match form {
            Form::Value => self.owned_type(&field.ty),
            Form::Reference => self.cow_type(&field.ty),
            Form::Ref => self.reference(&field.ty),
        };

        if field.field.optional {
            quote!(::std::option::Option<#ty>)
        } else {
            ty
        }
    }

    pub fn value(&self, value: &Value, ty: &Encoding<'_>) -> TokenStream2 {
        let core = self.core;

        match (value, ty) {
            (Value::Null, _) => quote!(::std::option::Option::None),
            (_, Encoding::Nullable(inner)) => {
                let inner = self.value(value, inner);

                quote!(::std::option::Option::Some(#inner))
            }
            (Value::Bool(value), _) => quote!(#value),
            (Value::Char(value), _) => quote!(#value),
            (Value::Text(value), _) => quote!(::std::convert::Into::into(#value)),
            (Value::Integer(value), Encoding::Number(NumberKind::Fixed32, _)) => {
                let value = Literal::u32_unsuffixed(Self::fitting(*value));

                quote!(#core::Fixed32(#value))
            }
            (Value::Integer(value), Encoding::Number(NumberKind::Fixed64, _)) => {
                let value = Literal::u64_unsuffixed(Self::fitting(*value));

                quote!(#core::Fixed64(#value))
            }
            (Value::Integer(value), Encoding::Number(kind, _)) => Self::integer(*value, *kind),
            (Value::Float(value), Encoding::Number(NumberKind::F32, _)) => {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "an f32 literal rounds to the nearest f32, as Rust's own does"
                )]
                let value = Literal::f32_suffixed(*value as f32);

                quote!(#value)
            }
            (Value::Float(value), _) => {
                let value = Literal::f64_suffixed(*value);

                quote!(#value)
            }
            (Value::Member { name, .. }, Encoding::Enumeration(enumeration)) => {
                let ty = self.declared_type(enumeration);
                let member = NameProvider::ident(name);

                quote!(#ty::#member)
            }
            _ => unreachable!("the constraints tie values to their types"),
        }
    }

    pub fn constant(&self, ty: &Encoding<'_>) -> TokenStream2 {
        match ty {
            Encoding::String(_) => quote!(&'static str),
            _ => self.owned_type(ty),
        }
    }

    pub fn constant_value(&self, value: &Value, ty: &Encoding<'_>) -> TokenStream2 {
        match value {
            Value::Text(text) => quote!(#text),
            _ => self.value(value, ty),
        }
    }

    /// Casts `value` to the integer type used to validate its literal.
    fn fitting<T: TryFrom<i128>>(value: i128) -> T {
        T::try_from(value)
            .ok()
            .expect("the constraints keep a literal within its type")
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "an integer literal of a float type rounds to the nearest float, as Rust's own does"
    )]
    fn integer(value: i128, kind: NumberKind) -> TokenStream2 {
        let literal = match kind {
            NumberKind::U8 => Literal::u8_suffixed(Self::fitting(value)),
            NumberKind::U16 => Literal::u16_suffixed(Self::fitting(value)),
            NumberKind::U32 => Literal::u32_suffixed(Self::fitting(value)),
            NumberKind::U64 => Literal::u64_suffixed(Self::fitting(value)),
            NumberKind::I8 => Literal::i8_suffixed(Self::fitting(value)),
            NumberKind::I16 => Literal::i16_suffixed(Self::fitting(value)),
            NumberKind::I32 => Literal::i32_suffixed(Self::fitting(value)),
            NumberKind::I64 => Literal::i64_suffixed(Self::fitting(value)),
            NumberKind::F32 => Literal::f32_suffixed(value as f32),
            NumberKind::F64 => Literal::f64_suffixed(value as f64),
            NumberKind::Fixed32 | NumberKind::Fixed64 => unreachable!("fixed integers wrap"),
        };

        quote!(#literal)
    }
}
