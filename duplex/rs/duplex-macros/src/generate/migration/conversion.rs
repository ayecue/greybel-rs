use duplex_protocol::{
    Direction, Encoding, FieldsPlan, PayloadPlan, Symbol, TypeContent, TypePlan,
};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::Ident;

use super::{MigrationBuilder, Side};
use crate::generate::names::NameProvider;

impl MigrationBuilder<'_> {
    pub(super) fn declaration<'s>(side: &'s Side<'_>, name: &str) -> Option<&'s TypePlan<'s>> {
        side.plan.types.iter().find(|plan| plan.name() == name)
    }

    pub(super) fn is_plain(ty: &Encoding<'_>) -> bool {
        match ty {
            Encoding::List(item, _) | Encoding::Record(item, _) | Encoding::Nullable(item) => {
                Self::is_plain(item)
            }
            Encoding::Struct(_) | Encoding::Union(_) | Encoding::Enumeration(_) => false,
            _ => true,
        }
    }

    pub(super) fn convert(
        &self,
        direction: Direction,
        from: &Encoding<'_>,
        to: &Encoding<'_>,
        value: &TokenStream2,
        borrowed: bool,
        conversions: &Ident,
    ) -> TokenStream2 {
        if Self::is_plain(from) {
            return value.clone();
        }

        match (from, to) {
            (Encoding::List(from, _), Encoding::List(to, _)) => {
                let item = self.convert(direction, from, to, &quote!(item), false, conversions);

                if borrowed {
                    quote! {
                        ::std::borrow::Cow::Owned(
                            #value.into_owned().into_iter().map(|item| #item).collect(),
                        )
                    }
                } else {
                    quote!(#value.into_iter().map(|item| #item).collect())
                }
            }
            (Encoding::Record(from, _), Encoding::Record(to, _)) => {
                let item = self.convert(direction, from, to, &quote!(item), false, conversions);

                quote!(#value.into_iter().map(|(key, item)| (key, #item)).collect())
            }
            (Encoding::Nullable(from), Encoding::Nullable(to)) => {
                let item = self.convert(direction, from, to, &quote!(item), borrowed, conversions);

                quote!(#value.map(|item| #item))
            }
            (from, to) => {
                let (Some(from_name), Some(to_name)) = (from.declared_name(), to.declared_name())
                else {
                    return value.clone();
                };
                let function = self.conversion(direction, from_name, to_name, conversions);

                quote!(#conversions::#function(#value))
            }
        }
    }

    pub(super) fn convert_fields(
        &self,
        direction: Direction,
        from: &FieldsPlan<'_>,
        to: &FieldsPlan<'_>,
        access: &dyn Fn(&Ident) -> TokenStream2,
        borrowed: bool,
        conversions: &Ident,
    ) -> TokenStream2 {
        let fields = from.fields().zip(to.fields()).map(|(from, to)| {
            let from_ident = NameProvider::field(from.field);
            let to_ident = NameProvider::field(to.field);
            let access = access(&from_ident);
            let conversion = if from.field.optional && !Self::is_plain(&from.ty) {
                let item = self.convert(
                    direction,
                    &from.ty,
                    &to.ty,
                    &quote!(item),
                    borrowed,
                    conversions,
                );

                quote!(#access.map(|item| #item))
            } else {
                self.convert(direction, &from.ty, &to.ty, &access, borrowed, conversions)
            };

            quote! { #to_ident: #conversion, }
        });
        let left_out = to.fields().skip(from.fields().count()).map(|field| {
            let ident = NameProvider::field(field.field);
            let types = &self.side(!direction.from_newer()).types;
            let value = match &field.default {
                Some(default) => types.value(default, &field.ty),
                None => quote!(::std::option::Option::None),
            };

            quote! { #ident: #value, }
        });

        quote! { #(#fields)* #(#left_out)* }
    }

    pub(super) fn conversion(
        &self,
        direction: Direction,
        from: &str,
        to: &str,
        conversions: &Ident,
    ) -> Ident {
        let prefix = match direction {
            Direction::Up => "up",
            Direction::Down => "down",
        };
        let name = Symbol::Declared(from).name().snake();
        let function = format_ident!("{prefix}_{name}");
        let key = (direction, from.to_owned(), to.to_owned());

        if self.conversions.borrow().contains_key(&key) {
            return function;
        }

        self.conversions
            .borrow_mut()
            .insert(key.clone(), TokenStream2::new());

        let (from_side, to_side) = match direction {
            Direction::Up => (&self.older, &self.newer),
            Direction::Down => (&self.newer, &self.older),
        };
        let from_path = from_side.path;
        let to_path = to_side.path;
        let from_ident = NameProvider::pascal(Symbol::Declared(from));
        let to_ident = NameProvider::pascal(Symbol::Declared(to));
        let (Some(from_plan), Some(to_plan)) = (
            Self::declaration(from_side, from),
            Self::declaration(to_side, to),
        ) else {
            return function;
        };
        let body = match (&from_plan.content, &to_plan.content) {
            (TypeContent::Struct(from_fields), TypeContent::Struct(to_fields)) => {
                let fields = self.convert_fields(
                    direction,
                    from_fields,
                    to_fields,
                    &|field| quote!(value.#field),
                    false,
                    conversions,
                );

                quote!(#to_path::#to_ident { #fields })
            }
            (TypeContent::Union(from_variants), TypeContent::Union(to_variants)) => {
                let union_cases = from_variants.iter().zip(to_variants).map(|(from, to)| {
                    let from_variant = NameProvider::pascal(Symbol::Declared(&from.variant.name));
                    let to_variant = NameProvider::pascal(Symbol::Declared(&to.variant.name));

                    match (&from.payload, &to.payload) {
                        (PayloadPlan::Value(from_value), PayloadPlan::Value(to_value)) => {
                            let conversion = self.convert(
                                direction,
                                from_value,
                                to_value,
                                &quote!(value),
                                false,
                                conversions,
                            );

                            quote! {
                                #from_path::#from_ident::#from_variant(value) => {
                                    #to_path::#to_ident::#to_variant(#conversion)
                                }
                            }
                        }
                        (PayloadPlan::Fields(from_fields), PayloadPlan::Fields(to_fields)) => {
                            let bindings = from_fields.fields().map(|field| NameProvider::field(field.field));
                            let fields = self.convert_fields(
                                direction,
                                from_fields,
                                to_fields,
                                &|field| quote!(#field),
                                false,
                                conversions,
                            );

                            quote! {
                                #from_path::#from_ident::#from_variant { #(#bindings),* } => {
                                    #to_path::#to_ident::#to_variant { #fields }
                                }
                            }
                        }
                        _ => quote! {
                            #from_path::#from_ident::#from_variant => #to_path::#to_ident::#to_variant
                        },
                    }
                });

                quote! { match value { #(#union_cases),* } }
            }
            (TypeContent::Enumeration(from_members), TypeContent::Enumeration(to_members)) => {
                let enumeration_cases = from_members.iter().zip(to_members).map(|(from, to)| {
                    let from_member = NameProvider::pascal(Symbol::Declared(&from.member.name));
                    let to_member = NameProvider::pascal(Symbol::Declared(&to.member.name));

                    quote!(#from_path::#from_ident::#from_member => #to_path::#to_ident::#to_member)
                });

                quote! { match value { #(#enumeration_cases),* } }
            }
            _ => return function,
        };
        let from_type = from_side.types.declared_type(from);
        let to_type = to_side.types.declared_type(to);

        self.conversions.borrow_mut().insert(
            key,
            quote! {
                fn #function(value: #from_type) -> #to_type {
                    #body
                }
            },
        );

        function
    }
}
