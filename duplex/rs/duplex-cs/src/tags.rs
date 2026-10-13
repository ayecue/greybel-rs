//! C# tag metadata, field metadata, and field-based class factory lambda generator.

use duplex_protocol::{Encoding, FieldsPlan, NumberKind, Plan, Tag, TagList, TypeRef, Value};

use crate::codec::{Property, Shape};
use crate::types;

const NO_TAGS: &str = "global::System.Array.Empty<global::Duplex.Tag>()";
const NO_FIELDS: &str = "global::System.Array.Empty<global::Duplex.FieldType>()";

pub(crate) struct TagWriter<'p, 'd> {
    plan: &'p Plan<'d>,
}

impl<'p, 'd> TagWriter<'p, 'd> {
    pub(crate) fn new(plan: &'p Plan<'d>) -> Self {
        Self { plan }
    }

    /// Converts `tags` to an `IReadOnlyList<Tag>` expression.
    pub(crate) fn tags(&self, tags: &TagList) -> String {
        if tags.is_empty() {
            return NO_TAGS.to_owned();
        }

        let tags: Vec<String> = tags.all().iter().map(|tag| self.tag(tag)).collect();

        format!("new global::Duplex.Tag[] {{ {} }}", tags.join(", "))
    }

    fn tag(&self, tag: &Tag) -> String {
        let name = types::string(&tag.name);
        let definition = self.plan.definition;
        let ty = definition
            .tag_declaration(&tag.name)
            .and_then(|declaration| declaration.ty.as_ref());

        match (definition.tag_value(tag), ty) {
            (Some(value), Some(ty)) => format!(
                "new global::Duplex.Tag({name}, {})",
                Self::literal(&value, ty)
            ),
            _ => format!("new global::Duplex.Tag({name})"),
        }
    }

    fn literal(value: &Value, ty: &TypeRef) -> String {
        match (value, ty) {
            (Value::Integer(value), TypeRef::Number(kind, _)) => types::integer(*value, *kind),
            (Value::Float(value), TypeRef::Number(NumberKind::F32, _)) => {
                format!("{}f", types::float(*value))
            }
            (Value::Member { name: member, .. }, TypeRef::Declared(name)) => {
                format!("{}.{}", types::pascal(name), types::pascal(member))
            }
            _ => types::literal(value, &Encoding::Bool),
        }
    }

    /// Returns `FieldType` metadata for `owner` fields in constructor parameter order.
    pub(crate) fn fields(&self, owner: &str, fields: &FieldsPlan<'_>) -> String {
        let properties: Vec<Property<'_, '_>> = fields
            .properties()
            .into_iter()
            .map(|field| Property::of(field, owner))
            .collect();

        if properties.is_empty() {
            return NO_FIELDS.to_owned();
        }

        let fields: Vec<String> = properties
            .iter()
            .map(|property| {
                Self::field(
                    &property.plan.field.name,
                    &Self::runtime_type(property),
                    &self.tags(&property.plan.field.tags),
                    owner,
                    &property.name,
                )
            })
            .collect();

        format!("new global::Duplex.FieldType[] {{ {} }}", fields.join(", "))
    }

    /// Returns `value` field metadata for a single-value union variant.
    pub(crate) fn value_fields(owner: &str, encoding: &Encoding<'_>) -> String {
        format!(
            "new global::Duplex.FieldType[] {{ {} }}",
            Self::field("value", &Self::runtime(encoding), NO_TAGS, owner, "Value")
        )
    }

    pub(crate) fn no_fields() -> &'static str {
        NO_FIELDS
    }

    fn field(name: &str, ty: &str, tags: &str, owner: &str, property: &str) -> String {
        format!(
            "new global::Duplex.FieldType({}, typeof({ty}), {tags}, holder => (({owner})holder).{property})",
            types::string(name)
        )
    }

    /// Returns a lambda constructing `owner` from an `object?[]` with one value per field.
    pub(crate) fn create(owner: &str, fields: &FieldsPlan<'_>) -> String {
        let arguments: Vec<String> = fields
            .properties()
            .into_iter()
            .enumerate()
            .map(|(index, field)| {
                let ty = Property::of(field, owner).ty();
                let suffix = if ty.ends_with('?') { "" } else { "!" };

                format!("({ty})values[{index}]{suffix}")
            })
            .collect();

        format!("values => new {owner}({})", arguments.join(", "))
    }

    /// Returns a property runtime type without reference-type nullable annotations unsupported by `typeof`.
    fn runtime_type(property: &Property<'_, '_>) -> String {
        let inner = property.inner();
        let ty = Self::runtime(inner);
        let nullable = if types::is_value_type(inner) {
            format!("{ty}?")
        } else {
            ty.clone()
        };

        match property.shape() {
            Shape::Always => ty,
            Shape::Present | Shape::NonNull => nullable,
            Shape::Both => format!("global::Duplex.Optional<{nullable}>"),
        }
    }

    fn runtime(encoding: &Encoding<'_>) -> String {
        match encoding {
            Encoding::Nullable(inner) if types::is_value_type(inner) => {
                format!("{}?", Self::runtime(inner))
            }
            Encoding::Nullable(inner) => Self::runtime(inner),
            Encoding::List(item, _) => format!(
                "global::System.Collections.Generic.IReadOnlyList<{}>",
                Self::runtime(item)
            ),
            Encoding::Record(item, _) => format!(
                "global::System.Collections.Generic.IReadOnlyDictionary<string, {}>",
                Self::runtime(item)
            ),
            other => types::of(other),
        }
    }
}
