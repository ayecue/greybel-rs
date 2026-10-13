use std::collections::HashSet;

use super::{Definition, Field, NumberKind, Payload, TypeKind, TypeRef};

const BOOL: u64 = 1;
const CHAR: u64 = 3;
const FLAG_BITS: u64 = 8;
const VARUINT_BITS: u32 = 7;

impl Definition {
    /// Returns maximum encoded bytes for `ty`, or `None` if unbounded.
    #[must_use]
    pub fn max_size(&self, ty: &TypeRef) -> Option<u64> {
        self.max_size_in(ty, &mut HashSet::new())
    }

    /// Returns maximum encoded bytes for bounded fields and flag bytes, excluding unbounded fields.
    #[must_use]
    pub fn bounded_size(&self, fields: &[Field]) -> u64 {
        self.bounded_size_in(fields, &mut HashSet::new())
    }

    fn max_size_in<'d>(
        &'d self,
        ty: &'d TypeRef,
        path_names: &mut HashSet<&'d str>,
    ) -> Option<u64> {
        match ty {
            TypeRef::Bool => Some(BOOL),
            TypeRef::Char => Some(CHAR),
            TypeRef::String(requirements) => requirements.length.max.map(Self::with_length),
            TypeRef::Bytes(length) => length.max.map(Self::with_length),
            TypeRef::Number(kind, _) => Some(kind.max_size()),
            TypeRef::List(item, length) => {
                let max = length.max?;
                let item = self.max_size_in(item, path_names)?;

                Some(Self::varuint_size(u64::from(max)) + u64::from(max) * item)
            }
            TypeRef::Record(..) => None,
            TypeRef::Nullable(inner) => Some(BOOL + self.max_size_in(inner, path_names)?),
            TypeRef::Declared(name) => {
                let declaration = self.declaration(name)?;

                if !path_names.insert(name) {
                    return None;
                }

                let size = match &declaration.kind {
                    TypeKind::Struct(fields) => self.fields_size(fields, path_names),
                    TypeKind::Union(variants) => {
                        let mut most = 0;

                        for variant in variants {
                            let payload = match &variant.payload {
                                Payload::Unit => Some(0),
                                Payload::Value(ty) => self.max_size_in(ty, path_names),
                                Payload::Fields(fields) => self.fields_size(fields, path_names),
                            };

                            most = most.max(payload?);
                        }

                        Some(Self::varuint_size(variants.len() as u64) + most)
                    }
                    TypeKind::Enumeration(members) => {
                        let most = members
                            .iter()
                            .enumerate()
                            .map(|(index, member)| {
                                member.value.unwrap_or(
                                    u32::try_from(index).expect("members are fewer than 2^32"),
                                )
                            })
                            .max()
                            .unwrap_or(0);

                        Some(Self::varuint_size(u64::from(most)))
                    }
                };

                path_names.remove(name.as_str());

                size
            }
        }
    }

    fn fields_size<'d>(
        &'d self,
        fields: &'d [Field],
        path_names: &mut HashSet<&'d str>,
    ) -> Option<u64> {
        let mut size = Self::flag_bytes(fields);

        for field in fields {
            size += self.field_size(field, path_names)?;
        }

        Some(size)
    }

    fn bounded_size_in<'d>(
        &'d self,
        fields: &'d [Field],
        path_names: &mut HashSet<&'d str>,
    ) -> u64 {
        let flags = Self::flag_bytes(fields);

        fields.iter().fold(flags, |size, field| {
            size + self.field_size(field, path_names).unwrap_or(0)
        })
    }

    fn field_size<'d>(
        &'d self,
        field: &'d Field,
        path_names: &mut HashSet<&'d str>,
    ) -> Option<u64> {
        match field.ty.non_null() {
            TypeRef::Bool => Some(0),
            ty => self.max_size_in(ty, path_names),
        }
    }

    fn flag_bytes(fields: &[Field]) -> u64 {
        let bits: u64 = fields
            .iter()
            .map(|field| {
                u64::from(field.optional || field.default.is_some())
                    + u64::from(field.ty.is_nullable())
                    + u64::from(matches!(field.ty.non_null(), TypeRef::Bool))
            })
            .sum();

        bits.div_ceil(FLAG_BITS)
    }

    fn with_length(length: u32) -> u64 {
        Self::varuint_size(u64::from(length)) + u64::from(length)
    }

    fn varuint_size(value: u64) -> u64 {
        u64::from(
            (u64::BITS - value.leading_zeros())
                .div_ceil(VARUINT_BITS)
                .max(1),
        )
    }
}

impl NumberKind {
    /// Returns maximum encoded bytes for this numeric type.
    #[must_use]
    pub fn max_size(self) -> u64 {
        match self {
            Self::U8 | Self::I8 => 1,
            Self::U16 | Self::I16 => 3,
            Self::U32 | Self::I32 => 5,
            Self::U64 | Self::I64 => 10,
            Self::F32 | Self::Fixed32 => 4,
            Self::F64 | Self::Fixed64 => 8,
        }
    }
}
