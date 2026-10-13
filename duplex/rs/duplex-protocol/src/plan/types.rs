use super::{Encoding, FieldsPlan, Source};
use crate::model::{
    Constant, Definition, Member, Payload, TypeDeclaration, TypeKind, Value, Variant,
};

/// Resolved declared constant value and encoding.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstantPlan<'d> {
    pub constant: &'d Constant,
    /// Constant binary encoding.
    pub ty: Encoding<'d>,
    /// Resolved constant value.
    pub value: Value,
}

impl<'d> ConstantPlan<'d> {
    pub(crate) fn of(definition: &'d Definition, constant: &'d Constant) -> Self {
        Self {
            constant,
            ty: Encoding::of(definition, &constant.ty),
            value: definition
                .value_of(&constant.value, &constant.ty)
                .expect("the constraints tie constants to their types"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypePlan<'d> {
    /// Source type declaration.
    pub declaration: &'d TypeDeclaration,
    /// Resolved declaration contents.
    pub content: TypeContent<'d>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeContent<'d> {
    /// Struct field layout.
    Struct(FieldsPlan<'d>),
    /// Union variant plans.
    Union(Vec<VariantPlan<'d>>),
    /// Enumeration member plans.
    Enumeration(Vec<MemberPlan<'d>>),
}

/// Resolved union variant generation plan.
#[derive(Debug, Clone, PartialEq)]
pub struct VariantPlan<'d> {
    /// Source variant declaration.
    pub variant: &'d Variant,
    /// Encoded variant index.
    pub index: u32,
    /// Introducing minor version. Older writers cannot encode this variant.
    pub since: u16,
    /// Variant payload layout.
    pub payload: PayloadPlan<'d>,
}

/// Resolved union variant payload layout.
#[derive(Debug, Clone, PartialEq)]
pub enum PayloadPlan<'d> {
    /// Variant has no payload.
    Unit,
    /// One encoded value.
    Value(Encoding<'d>),
    /// Field layout.
    Fields(FieldsPlan<'d>),
}

/// Resolved enumeration member generation plan.
#[derive(Debug, Clone, PartialEq)]
pub struct MemberPlan<'d> {
    /// Source member declaration.
    pub member: &'d Member,
    /// Encoded member numeric value.
    pub value: u32,
    /// Introducing minor version. Older writers cannot encode this member.
    pub since: u16,
}

impl<'d> TypePlan<'d> {
    pub(crate) fn of(source: Source<'d>, declaration: &'d TypeDeclaration) -> Self {
        let content = match &declaration.kind {
            TypeKind::Struct(fields) => TypeContent::Struct(FieldsPlan::of(source, fields)),
            TypeKind::Union(variants) => TypeContent::Union(
                (0..)
                    .zip(variants)
                    .map(|(index, variant)| VariantPlan {
                        variant,
                        index,
                        since: source.additions.variant(variant),
                        payload: match &variant.payload {
                            Payload::Unit => PayloadPlan::Unit,
                            Payload::Value(ty) => {
                                PayloadPlan::Value(Encoding::of(source.definition, ty))
                            }
                            Payload::Fields(fields) => {
                                PayloadPlan::Fields(FieldsPlan::of(source, fields))
                            }
                        },
                    })
                    .collect(),
            ),
            TypeKind::Enumeration(members) => TypeContent::Enumeration(
                (0..)
                    .zip(members)
                    .map(|(index, member)| MemberPlan {
                        member,
                        value: member.value.unwrap_or(index),
                        since: source.additions.member(member),
                    })
                    .collect(),
            ),
        };

        Self {
            declaration,
            content,
        }
    }

    #[must_use]
    pub fn name(&self) -> &'d str {
        &self.declaration.name
    }
}
