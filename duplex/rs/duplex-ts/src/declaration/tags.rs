//! TypeScript tag and field metadata generator for generated classes.

use duplex_protocol::{Definition, FieldsPlan, NumberKind, Tag, TagList, TypeRef, Value};
use oxc_ast::ast::{ClassElement, Expression, TSType};
use oxc_span::SPAN;

use super::VALUE;
use crate::ast::{ModifierContainer, Ts};
use crate::types::TsTypeMapper;

pub(crate) const TAGS: &str = "tags";
pub(crate) const FIELDS: &str = "fields";
pub(crate) const MEMBERS: &str = "members";
pub(crate) const TAG: &str = "Tag";
pub(crate) const FIELD_TAGS: &str = "FieldTagContainer";
pub(crate) const MEMBER_TAGS: &str = "MemberTagContainer";

pub(crate) struct TagExpressionContainer<'t, 'a> {
    ts: &'t Ts<'a>,
    definition: &'t Definition,
}

impl<'t, 'a> TagExpressionContainer<'t, 'a> {
    pub(crate) fn new(ts: &'t Ts<'a>, definition: &'t Definition) -> Self {
        Self { ts, definition }
    }

    /// Converts `tags` to `{ name, value }` objects; flag tags omit `value`.
    pub(crate) fn tags(&self, tags: &TagList) -> Expression<'a> {
        let ts = self.ts;

        ts.array_literal(
            tags.all()
                .iter()
                .map(|tag| {
                    let mut properties = vec![("name", ts.string_literal(&tag.name))];

                    properties.extend(self.value(tag).map(|value| ("value", value)));

                    ts.object(properties)
                })
                .collect(),
        )
    }

    fn value(&self, tag: &Tag) -> Option<Expression<'a>> {
        let ts = self.ts;
        let ty = self.definition.tag_declaration(&tag.name)?.ty.as_ref()?;

        Some(match (self.definition.tag_value(tag)?, ty) {
            (
                Value::Integer(value),
                TypeRef::Number(NumberKind::U64 | NumberKind::I64 | NumberKind::Fixed64, _),
            ) => ts.bigint_literal(value),
            #[allow(clippy::cast_precision_loss)]
            (Value::Integer(value), _) => ts.number_literal(value as f64),
            (Value::Float(value), _) => ts.number_literal(value),
            (Value::Bool(value), _) => ts.boolean_literal(value),
            (Value::Text(text), _) => ts.string_literal(&text),
            (Value::Char(char), _) => ts.string_literal(&char.to_string()),

            (Value::Member { value, .. }, _) => ts.number_literal(value),
            (Value::Null, _) => return None,
        })
    }

    /// Returns fields as `{ name, property, tags }` metadata objects with generated property names.
    pub(crate) fn fields(&self, fields: &FieldsPlan<'_>) -> Expression<'a> {
        let ts = self.ts;

        ts.array_literal(
            fields
                .properties()
                .into_iter()
                .map(|field| {
                    Self::field(
                        ts,
                        &field.field.name,
                        &TsTypeMapper::property(field.field),
                        self.tags(&field.field.tags),
                    )
                })
                .collect(),
        )
    }

    /// Returns `value` field metadata for a single-value variant.
    pub(crate) fn value_fields(&self) -> Expression<'a> {
        let ts = self.ts;

        ts.array_literal(vec![Self::field(
            ts,
            VALUE,
            VALUE,
            ts.array_literal(Vec::new()),
        )])
    }

    fn field(ts: &Ts<'a>, name: &str, property: &str, tags: Expression<'a>) -> Expression<'a> {
        ts.object(vec![
            ("name", ts.string_literal(name)),
            ("property", ts.string_literal(property)),
            ("tags", tags),
        ])
    }

    /// Returns static readonly `tags` and `fields` properties with `override` when `is_override` is true.
    pub(crate) fn statics(
        &self,
        tags: &TagList,
        fields: Expression<'a>,
        is_override: bool,
    ) -> Vec<ClassElement<'a>> {
        let ts = self.ts;
        let modifiers = ModifierContainer::STATIC_READONLY.with_override(is_override);

        vec![
            ts.property(
                SPAN,
                TAGS,
                Some(self.list(TAG)),
                Some(self.tags(tags)),
                modifiers,
            ),
            ts.property(
                SPAN,
                FIELDS,
                Some(self.list(FIELD_TAGS)),
                Some(fields),
                modifiers,
            ),
        ]
    }

    /// Creates type `ReadonlyArray<item>`.
    pub(crate) fn list(&self, item: &str) -> TSType<'a> {
        let ts = self.ts;

        ts.ty("ReadonlyArray", vec![ts.ty(item, Vec::new())])
    }
}
