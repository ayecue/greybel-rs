use super::{KeyHash, Text, Value};

#[derive(Debug, Clone)]
pub struct Key {
    pub(crate) value: Value,
    pub(crate) hash: KeyHash,
}

impl Key {
    #[must_use]
    pub fn new(text: impl Into<Text>) -> Self {
        let text = text.into();

        Self {
            hash: KeyHash::of_text(&text),
            value: Value::String(text),
        }
    }

    #[must_use]
    pub fn text(&self) -> &str {
        match &self.value {
            Value::String(text) => text,
            _ => "",
        }
    }

    #[must_use]
    pub fn value(&self) -> &Value {
        &self.value
    }
}
