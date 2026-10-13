use super::{Encoding, Source};
use crate::model::{Field, TypeKind, TypeRef, Value};

const BYTE_BITS: usize = 8;

/// Flag bit location in a field block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bit {
    pub byte: usize,
    /// Bit mask within the flag byte.
    pub mask: u8,
}

/// Encoding blocks grouped by introducing minor version.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldsPlan<'d> {
    /// Field blocks in encoding order.
    pub blocks: Vec<BlockPlan<'d>>,
}

/// Flag bytes and non-flag values for fields from one minor version.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockPlan<'d> {
    /// Introducing minor version, or 0 for initial major-version fields.
    pub since: u16,
    pub fields: Vec<FieldPlan<'d>>,
    /// Defined flag-bit masks. Readers reject unknown bits.
    pub flags: Vec<u8>,
}

/// Resolved field binary layout.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldPlan<'d> {
    pub field: &'d Field,
    pub position: usize,
    pub ty: Encoding<'d>,
    /// Field presence and nullability flag bits.
    pub presence: Presence,
    pub value: FieldValue<'d>,
    /// Value used when the sender omits this field block.
    pub default: Option<Value>,
}

/// Optional presence and non-null flags controlling field encoding.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Presence {
    pub present: Option<Bit>,
    pub non_null: Option<Bit>,
}

impl FieldPlan<'_> {
    #[must_use]
    pub fn can_be_left_out(&self) -> bool {
        self.field.optional || self.default.is_some()
    }
}

impl Presence {
    #[must_use]
    pub fn is_required(self) -> bool {
        self.present.is_none() && self.non_null.is_none()
    }
}

/// Field representation in the containing binary layout.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldValue<'d> {
    /// Boolean in flag bytes.
    Flag(Bit),
    /// Non-Boolean value after flag bytes.
    Value(Encoding<'d>),
    /// Inline struct fields in the containing layout.
    Flatten {
        name: &'d str,
        fields: FieldsPlan<'d>,
    },
}

impl<'d> FieldsPlan<'d> {
    pub(crate) fn of(source: Source<'d>, fields: &'d [Field]) -> Self {
        let mut blocks: Vec<BlockPlan<'d>> = Vec::new();
        let mut start = 0;

        while start < fields.len() {
            let since = source.additions.field(&fields[start]);
            let end = fields[start..]
                .iter()
                .position(|field| source.additions.field(field) != since)
                .map_or(fields.len(), |end| start + end);

            blocks.push(BlockPlan::of(source, since, &fields[start..end], start));
            start = end;
        }

        Self { blocks }
    }

    fn flatten(source: Source<'d>, ty: &'d TypeRef) -> FieldValue<'d> {
        let TypeRef::Declared(name) = ty else {
            unreachable!("the constraints flatten only structs");
        };
        let Some(TypeKind::Struct(fields)) = source.definition.declaration(name).map(|it| &it.kind)
        else {
            unreachable!("the constraints flatten only structs");
        };

        FieldValue::Flatten {
            name,
            fields: Self::of(source, fields),
        }
    }

    pub fn fields(&self) -> impl Iterator<Item = &FieldPlan<'d>> {
        self.blocks.iter().flat_map(|block| &block.fields)
    }

    #[must_use]
    pub fn properties(&self) -> Vec<&FieldPlan<'d>> {
        let mut properties = Vec::new();

        for field in self.fields() {
            match &field.value {
                FieldValue::Flatten { fields, .. } => properties.extend(fields.properties()),
                _ => properties.push(field),
            }
        }

        properties
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}

impl<'d> BlockPlan<'d> {
    fn of(source: Source<'d>, since: u16, fields: &'d [Field], first: usize) -> Self {
        let mut bits = 0;
        let mut next = || {
            let bit = Bit {
                byte: bits / BYTE_BITS,
                mask: 1 << (bits % BYTE_BITS),
            };

            bits += 1;

            bit
        };
        let fields: Vec<FieldPlan<'d>> = fields
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let position = first + index;
                let ty = Encoding::of(source.definition, &field.ty);

                if field.flatten {
                    return FieldPlan {
                        field,
                        position,
                        presence: Presence::default(),
                        value: FieldsPlan::flatten(source, &field.ty),
                        ty,
                        default: None,
                    };
                }

                let presence = Presence {
                    present: field.optional.then(&mut next),
                    non_null: field.ty.is_nullable().then(&mut next),
                };
                let value = match ty.non_null() {
                    Encoding::Bool => FieldValue::Flag(next()),
                    encoding => FieldValue::Value(encoding.clone()),
                };

                FieldPlan {
                    field,
                    position,
                    ty,
                    presence,
                    value,
                    default: source.definition.default_of(field),
                }
            })
            .collect();
        let mut flags = vec![0; bits.div_ceil(BYTE_BITS)];

        for field in &fields {
            let flag = match field.value {
                FieldValue::Flag(bit) => Some(bit),
                _ => None,
            };

            for bit in [field.presence.present, field.presence.non_null, flag]
                .into_iter()
                .flatten()
            {
                flags[bit.byte] |= bit.mask;
            }
        }

        Self {
            since,
            fields,
            flags,
        }
    }
}
