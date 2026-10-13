use greybel_interpreter::{Heap, Value};

/// Maximum rendered value characters; longer values receive a trailing ellipsis.
const VALUE_LENGTH: usize = 512;

/// Named debugger value and optional expression for reading its scope or collection entry.
pub(super) struct Entry {
    pub(super) name: String,
    /// Optional expression for reading the value in the innermost frame.
    pub(super) evaluate_name: Option<String>,
    pub(super) value: Value,
}

impl Entry {
    /// Converts map entries to debugger entries with string/numeric-key accessors under optional `parent`.
    pub(super) fn of_map<'v>(
        heap: &Heap,
        entries: impl Iterator<Item = (&'v Value, &'v Value)>,
        parent: Option<&str>,
    ) -> Vec<Self> {
        entries
            .map(|(key, value)| {
                let (name, accessor) = match key {
                    Value::String(_) => {
                        let name = heap.display(key).to_string();
                        let accessor = Self::string_accessor(&name);

                        (name, Some(accessor))
                    }
                    Value::Number(_) => {
                        let name = Self::display(heap, key);
                        let accessor = format!("[{name}]");

                        (name, Some(accessor))
                    }
                    key => (Self::display(heap, key), None),
                };

                Self {
                    name,
                    evaluate_name: parent
                        .zip(accessor)
                        .map(|(parent, accessor)| format!("{parent}{accessor}")),
                    value: value.clone(),
                }
            })
            .collect()
    }

    /// Builds a MiniScript `["key"]` accessor, doubling embedded quotes.
    pub(super) fn string_accessor(key: &str) -> String {
        format!("[\"{}\"]", key.replace('"', "\"\""))
    }

    /// Renders values with quoted strings and doubled internal quotes; truncates after 512 characters with an ellipsis.
    pub(super) fn display(heap: &Heap, value: &Value) -> String {
        let text = match value {
            Value::String(text) => format!("\"{}\"", text.replace('"', "\"\"")),
            value => heap.display(value).to_string(),
        };

        match text.char_indices().nth(VALUE_LENGTH) {
            Some((end, _)) => format!("{}…", &text[..end]),
            None => text,
        }
    }
}
