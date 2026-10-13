use meta_utils::{ArgumentDefault, BaseType, TypeMeta, Variation};

use crate::protocol::{Literal, TypePart, TypeText};

impl TypeText {
    /// Converts `type_meta` to display text and separates names from punctuation for links and styles.
    pub(crate) fn of(type_meta: &TypeMeta) -> Self {
        let mut parts = Vec::new();

        Self::write(type_meta, &mut parts);

        Self { parts }
    }

    fn write(type_meta: &TypeMeta, parts: &mut Vec<TypePart>) {
        match type_meta {
            TypeMeta::Name(name) => parts.push(TypePart::Name { name: name.clone() }),
            TypeMeta::List(value) => {
                Self::name(BaseType::List, parts);
                Self::text("<", parts);
                Self::write(value, parts);
                Self::text(">", parts);
            }
            TypeMeta::Map { key, value } => {
                Self::name(BaseType::Map, parts);
                Self::text("<", parts);
                Self::write(key, parts);
                Self::text(",", parts);
                Self::write(value, parts);
                Self::text(">", parts);
            }
        }
    }

    fn name(base: BaseType, parts: &mut Vec<TypePart>) {
        parts.push(TypePart::Name {
            name: base.as_str().to_owned(),
        });
    }

    fn text(text: &str, parts: &mut Vec<TypePart>) {
        parts.push(TypePart::Text {
            text: text.to_owned(),
        });
    }
}

impl From<&ArgumentDefault> for Literal {
    fn from(default: &ArgumentDefault) -> Self {
        match default {
            ArgumentDefault::String(value) => Self::String {
                value: value.clone(),
            },
            ArgumentDefault::Number(value) => Self::Number { value: *value },
        }
    }
}

impl From<&Variation> for Literal {
    fn from(variation: &Variation) -> Self {
        match variation {
            Variation::Text(value) => Self::String {
                value: value.clone(),
            },
            Variation::Number(value) => Self::Number { value: *value },
        }
    }
}
