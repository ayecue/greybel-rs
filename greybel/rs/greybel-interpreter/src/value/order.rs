use std::cmp::Ordering;

use super::{Heap, Value};

impl Heap {
    #[must_use]
    pub fn order(&self, a: &Value, b: &Value) -> Ordering {
        match (a, b) {
            (Value::Null, Value::Null) => Ordering::Equal,
            (Value::Null, _) => Ordering::Greater,
            (_, Value::Null) => Ordering::Less,
            (Value::String(a), Value::String(b)) => mono_string::collation::compare(a, b),
            (Value::String(_), _) | (_, Value::String(_)) => mono_string::collation::compare(
                &self.display(a).to_string(),
                &self.display(b).to_string(),
            ),
            (Value::Number(a), Value::Number(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
            _ => Ordering::Equal,
        }
    }
}
