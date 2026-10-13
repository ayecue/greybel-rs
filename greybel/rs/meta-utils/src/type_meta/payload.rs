use serde::de::{self, Deserialize, Deserializer};
use serde::ser::{Serialize, SerializeStruct, Serializer};

use super::{BaseType, TypeMeta, parser};
use crate::error::TypeParseError;

#[derive(Debug, thiserror::Error)]
enum TypePayloadError {
    #[error("invalid type `{text}`: {source}")]
    Parse {
        text: String,
        #[source]
        source: TypeParseError,
    },
    #[error("invalid type name `{name}`")]
    Name { name: String },
    #[error("`{field}` does not apply to `{ty}`")]
    Field { field: &'static str, ty: String },
}

impl Serialize for TypeMeta {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Name(name) => serializer.serialize_str(name),
            Self::List(value) => {
                let mut object = serializer.serialize_struct("TypeMeta", 2)?;

                object.serialize_field("type", BaseType::List.as_str())?;
                object.serialize_field("valueType", value)?;

                object.end()
            }
            Self::Map { key, value } => {
                let mut object = serializer.serialize_struct("TypeMeta", 3)?;

                object.serialize_field("type", BaseType::Map.as_str())?;
                object.serialize_field("keyType", key)?;
                object.serialize_field("valueType", value)?;

                object.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for TypeMeta {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        TypePayload::deserialize(deserializer)?
            .into_meta()
            .map_err(de::Error::custom)
    }
}

#[derive(serde::Deserialize)]
#[serde(untagged, expecting = "a type name or a type object")]
enum TypePayload {
    Name(String),
    Object(TypeObject),
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct TypeObject {
    r#type: String,
    key_type: Option<Box<TypePayload>>,
    value_type: Option<Box<TypePayload>>,
}

impl TypePayload {
    fn into_meta(self) -> Result<TypeMeta, TypePayloadError> {
        let object = match self {
            Self::Name(text) => {
                return text.parse().map_err(|source| TypePayloadError::Parse {
                    text: text.clone(),
                    source,
                });
            }
            Self::Object(object) => object,
        };

        if !parser::is_name(&object.r#type) {
            return Err(TypePayloadError::Name {
                name: object.r#type.clone(),
            });
        }

        let content = |payload: Option<Box<Self>>| -> Result<Box<TypeMeta>, TypePayloadError> {
            payload
                .map_or_else(|| Ok(TypeMeta::any()), |payload| payload.into_meta())
                .map(Box::new)
        };
        let base = BaseType::from_name(&object.r#type);
        let takes_key = base == Some(BaseType::Map);
        let takes_value = matches!(base, Some(BaseType::List | BaseType::Map));
        let inapplicable_fields = [
            ("keyType", object.key_type.is_some() && !takes_key),
            ("valueType", object.value_type.is_some() && !takes_value),
        ];

        if let Some((field, _)) = inapplicable_fields.iter().find(|(_, present)| *present) {
            return Err(TypePayloadError::Field {
                field,
                ty: object.r#type.clone(),
            });
        }

        Ok(match base {
            Some(BaseType::List) => TypeMeta::List(content(object.value_type)?),
            Some(BaseType::Map) => TypeMeta::Map {
                key: content(object.key_type)?,
                value: content(object.value_type)?,
            },
            _ => TypeMeta::Name(object.r#type),
        })
    }
}
