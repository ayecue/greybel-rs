use std::fmt;

use greybel_symbols::literals::NULL;

use super::{Function, Heap, Value};

const MAX_DEPTH: u32 = 2;
const TRUNCATED_MAP: &str = "{...}";
const TRUNCATED_LIST: &str = "[...]";
const FUNCTION_START: &str = "FUNCTION(";
const LIST_SEPARATOR: &str = ", ";
const MAP_SEPARATOR: &str = ", ";
const KEY_SEPARATOR: &str = ": ";
const PARAM_SEPARATOR: &str = ", ";

#[derive(Clone, Copy)]
pub struct ValueDisplay<'a> {
    heap: &'a Heap,
    value: &'a Value,
    with_quotes: bool,
}

impl ValueDisplay<'_> {
    #[must_use]
    pub fn with_quotes(mut self) -> Self {
        self.with_quotes = true;

        self
    }

    fn write(&self, f: &mut fmt::Formatter<'_>, value: &Value, depth: u32) -> fmt::Result {
        let heap = self.heap;

        match value {
            Value::Null => f.write_str(NULL),
            Value::Number(number) => fmt::Display::fmt(number, f),
            Value::String(text) => write!(f, "\"{text}\""),
            Value::Function(id) => self.write_function(f, *id),
            Value::List(id) => {
                if depth > MAX_DEPTH {
                    return f.write_str(TRUNCATED_LIST);
                }

                f.write_str("[")?;
                for (index, item) in id.items(heap).iter().enumerate() {
                    if index > 0 {
                        f.write_str(LIST_SEPARATOR)?;
                    }

                    self.write(f, item, depth + 1)?;
                }

                f.write_str("]")
            }
            Value::Map(id) => {
                if let Some(label) = heap.map_label().and_then(|label| label(heap, *id)) {
                    return f.write_str(&label);
                }

                if depth > MAX_DEPTH {
                    return f.write_str(TRUNCATED_MAP);
                }

                f.write_str("{")?;
                for (index, (key, item)) in id.entries(heap).iter().enumerate() {
                    if index > 0 {
                        f.write_str(MAP_SEPARATOR)?;
                    }

                    self.write(f, key, depth + 1)?;
                    f.write_str(KEY_SEPARATOR)?;
                    self.write(f, item, depth + 1)?;
                }

                f.write_str("}")
            }
        }
    }

    fn write_function(&self, f: &mut fmt::Formatter<'_>, id: Function) -> fmt::Result {
        f.write_str(FUNCTION_START)?;
        for (index, param) in id.closure(self.heap).params.iter().enumerate() {
            if index > 0 {
                f.write_str(PARAM_SEPARATOR)?;
            }

            f.write_str(&param.name)?;

            match &param.default {
                Value::Null => {}
                default => {
                    f.write_str("=")?;
                    self.write(f, default, 0)?;
                }
            }
        }

        f.write_str(")")
    }
}

impl fmt::Display for ValueDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.value {
            Value::String(text) if !self.with_quotes => f.write_str(text),
            value => self.write(f, value, 0),
        }
    }
}

impl Heap {
    #[must_use]
    pub fn display<'a>(&'a self, value: &'a Value) -> ValueDisplay<'a> {
        ValueDisplay {
            heap: self,
            value,
            with_quotes: false,
        }
    }
}
