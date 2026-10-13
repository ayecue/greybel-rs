use duplex_protocol::{Definition, Encoding, Length, PatternTable, Value};
use proc_macro2::{Literal, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::Ident;

use super::{Form, RustTypeMapper};

impl RustTypeMapper<'_> {
    pub fn patterns(&self) -> TokenStream2 {
        let core = self.core;
        let patterns = self
            .plan
            .patterns()
            .into_iter()
            .enumerate()
            .map(|(index, source)| {
                let table =
                    PatternTable::compile(source).expect("the constraints compile every pattern");
                let ident = Self::pattern(index);
                let PatternTable {
                    classes,
                    start,
                    base,
                    next,
                    owner,
                    accepting,
                } = table;

                quote! {
                    #[doc(hidden)]
                    static #ident: #core::Pattern = #core::Pattern {
                        classes: [#(#classes),*],
                        start: #start,
                        base: &[#(#base),*],
                        next: &[#(#next),*],
                        owner: &[#(#owner),*],
                        accepting: &[#(#accepting),*],
                    };
                }
            });

        quote! { #(#patterns)* }
    }

    pub fn requires(&self, encoding: &Encoding<'_>) -> bool {
        self.plan.requires(encoding)
    }

    pub fn validation(&self, encoding: &Encoding<'_>) -> TokenStream2 {
        let core = self.core;

        match encoding {
            Encoding::String(requirements) => {
                let pattern = match &requirements.pattern {
                    Some(pattern) => {
                        let index = self
                            .plan
                            .patterns()
                            .iter()
                            .position(|source| source == pattern)
                            .expect("the plan lists every pattern");
                        let ident = Self::pattern(index);

                        quote!(::std::option::Option::Some(&#ident))
                    }
                    None => quote!(::std::option::Option::None),
                };

                Self::length(core, requirements.length, &pattern)
            }
            Encoding::Bytes(length) => {
                Self::length(core, *length, &quote!(::std::option::Option::None))
            }
            Encoding::Number(kind, bounds) if !bounds.is_any() => {
                let (min, max) =
                    Definition::bounds_of(*kind, bounds).expect("the constraints verify bounds");

                if kind.is_float() {
                    let bound = |value: Option<Value>, absent: TokenStream2| match value {
                        Some(Value::Float(value)) => {
                            let value = Literal::f64_suffixed(value);

                            quote!(#value)
                        }
                        _ => absent,
                    };
                    let (min, max) = (
                        bound(min, quote!(f64::NEG_INFINITY)),
                        bound(max, quote!(f64::INFINITY)),
                    );

                    return quote!(#core::Validation::Float { min: #min, max: #max });
                }

                let bound = |value: Option<Value>, absent: TokenStream2| match value {
                    Some(Value::Integer(value)) => {
                        let value = Literal::i128_suffixed(value);

                        quote!(#value)
                    }
                    _ => absent,
                };
                let (min, max) = (bound(min, quote!(i128::MIN)), bound(max, quote!(i128::MAX)));

                quote!(#core::Validation::Integer { min: #min, max: #max })
            }
            Encoding::List(item, length) | Encoding::Record(item, length) => {
                Self::items(core, *length, &self.validation(item))
            }
            Encoding::Nullable(inner) => self.validation(inner),
            _ => quote!(#core::Validation::Any),
        }
    }

    pub fn items(core: &TokenStream2, length: Length, item: &TokenStream2) -> TokenStream2 {
        let (min, max) = Self::bounds(length);

        quote!(#core::Validation::Items { min: #min, max: #max, item: &#item })
    }

    pub fn validated_read(
        &self,
        encoding: &Encoding<'_>,
        position: Option<usize>,
        form: Form,
    ) -> TokenStream2 {
        let core = self.core;
        let borrows = form == Form::Ref;

        if !self.requires(encoding) {
            return if borrows {
                quote!(#core::BorrowDecode::read_ref(reader)?)
            } else {
                quote!(#core::BinaryDecode::read(reader)?)
            };
        }

        let validation = self.validation(encoding);
        let invalid_field = Self::invalid_field(position);
        let read = if borrows {
            quote!(#core::BorrowDecode::read_ref_with_validation(reader, &VALIDATION))
        } else {
            quote!(#core::ValidatedDecode::read_with_validation(reader, &VALIDATION))
        };

        quote! {{
            const VALIDATION: #core::Validation = #validation;
            #read #invalid_field?
        }}
    }

    pub fn verify(
        &self,
        value: &TokenStream2,
        encoding: &Encoding<'_>,
        position: Option<usize>,
    ) -> Option<TokenStream2> {
        if !self.requires(encoding) {
            return None;
        }

        let core = self.core;
        let validation = self.validation(encoding);
        let invalid_field = Self::invalid_field(position);

        Some(quote! {{
            const VALIDATION: #core::Validation = #validation;
            #core::Validatable::validate(#value, &VALIDATION)#invalid_field?;
        }})
    }

    fn invalid_field(position: Option<usize>) -> Option<TokenStream2> {
        position.map(|position| quote!(.map_err(|error| error.invalid_field(#position))))
    }

    fn length(core: &TokenStream2, length: Length, pattern: &TokenStream2) -> TokenStream2 {
        let (min, max) = Self::bounds(length);

        quote!(#core::Validation::Length { min: #min, max: #max, pattern: #pattern })
    }

    fn bounds(length: Length) -> (u32, u32) {
        (length.min, length.max.unwrap_or(u32::MAX))
    }

    fn pattern(index: usize) -> Ident {
        format_ident!("__DUPLEX_PATTERN_{}", index)
    }
}
