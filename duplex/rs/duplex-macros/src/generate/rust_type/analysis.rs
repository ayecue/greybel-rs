use duplex_protocol::{Encoding, FieldsPlan, PayloadPlan, TypeContent};

use super::RustTypeMapper;

impl RustTypeMapper<'_> {
    pub fn borrows(fields: &FieldsPlan<'_>) -> bool {
        fields.fields().any(|field| {
            matches!(
                field.ty.non_null(),
                Encoding::String(_) | Encoding::List(..)
            )
        })
    }

    /// Returns whether `ty` has no strings, bytes, or records and can decode without borrowing input.
    pub fn is_plain(&self, ty: &Encoding<'_>) -> bool {
        match ty {
            Encoding::Bool | Encoding::Char | Encoding::Number(..) => true,
            Encoding::String(_) | Encoding::Bytes(_) | Encoding::Record(..) => false,
            Encoding::List(inner, _) | Encoding::Nullable(inner) => self.is_plain(inner),
            Encoding::Struct(name) | Encoding::Union(name) | Encoding::Enumeration(name) => {
                self.all_of(name, Self::is_plain)
            }
        }
    }

    pub fn is_copy(&self, ty: &Encoding<'_>) -> bool {
        match ty {
            Encoding::Bool | Encoding::Char | Encoding::Number(..) => true,
            Encoding::String(_)
            | Encoding::Bytes(_)
            | Encoding::List(..)
            | Encoding::Record(..) => false,
            Encoding::Nullable(inner) => self.is_copy(inner),
            Encoding::Struct(name) | Encoding::Union(name) | Encoding::Enumeration(name) => {
                self.all_of(name, Self::is_copy)
            }
        }
    }

    pub fn is_eq(&self, ty: &Encoding<'_>) -> bool {
        match ty {
            Encoding::Bool | Encoding::Char | Encoding::String(_) | Encoding::Bytes(_) => true,
            Encoding::Number(kind, _) => !kind.is_float(),
            Encoding::List(item, _) | Encoding::Record(item, _) | Encoding::Nullable(item) => {
                self.is_eq(item)
            }
            Encoding::Struct(name) | Encoding::Union(name) | Encoding::Enumeration(name) => {
                self.all_of(name, Self::is_eq)
            }
        }
    }

    pub fn fields_are(
        &self,
        fields: &FieldsPlan<'_>,
        test: fn(&Self, &Encoding<'_>) -> bool,
    ) -> bool {
        fields.fields().all(|field| test(self, &field.ty))
    }

    pub fn content_is(
        &self,
        content: &TypeContent<'_>,
        test: fn(&Self, &Encoding<'_>) -> bool,
    ) -> bool {
        match content {
            TypeContent::Struct(fields) => self.fields_are(fields, test),
            TypeContent::Union(variants) => variants.iter().all(|variant| match &variant.payload {
                PayloadPlan::Unit => true,
                PayloadPlan::Value(ty) => test(self, ty),
                PayloadPlan::Fields(fields) => self.fields_are(fields, test),
            }),
            TypeContent::Enumeration(_) => true,
        }
    }

    pub(super) fn all_of(&self, name: &str, test: fn(&Self, &Encoding<'_>) -> bool) -> bool {
        let Some(plan) = self.plan.types.iter().find(|plan| plan.name() == name) else {
            return false;
        };

        if !self.path_names.borrow_mut().insert(plan.name()) {
            return true;
        }

        let passes = self.content_is(&plan.content, test);

        self.path_names.borrow_mut().remove(plan.name());

        passes
    }
}
