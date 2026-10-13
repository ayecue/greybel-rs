//! C# identifier, type name, and literal generation for protocol declarations.

use std::fmt::Write;

use duplex_protocol::{Encoding, Field, NumberKind, Symbol, Value};

/// C# keywords escaped with `@` in parameter and local names.
const KEYWORDS: [&str; 77] = [
    "abstract",
    "as",
    "base",
    "bool",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "checked",
    "class",
    "const",
    "continue",
    "decimal",
    "default",
    "delegate",
    "do",
    "double",
    "else",
    "enum",
    "event",
    "explicit",
    "extern",
    "false",
    "finally",
    "fixed",
    "float",
    "for",
    "foreach",
    "goto",
    "if",
    "implicit",
    "in",
    "int",
    "interface",
    "internal",
    "is",
    "lock",
    "long",
    "namespace",
    "new",
    "null",
    "object",
    "operator",
    "out",
    "override",
    "params",
    "private",
    "protected",
    "public",
    "readonly",
    "ref",
    "return",
    "sbyte",
    "sealed",
    "short",
    "sizeof",
    "stackalloc",
    "static",
    "string",
    "struct",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "uint",
    "ulong",
    "unchecked",
    "unsafe",
    "ushort",
    "using",
    "virtual",
    "void",
    "volatile",
    "while",
];

/// Reserved member names for generated properties.
const RESERVED_MEMBERS: [&str; 8] = [
    "Write",
    "Read",
    "MessageType",
    "GetMessageType",
    "Equals",
    "GetHashCode",
    "ToString",
    "GetType",
];

/// Property suffix for class-name and reserved-member conflicts.
const PROPERTY_SUFFIX: &str = "Field";

/// Returns the `PascalCase` C# name for a protocol, peer, message, type, variant, or member declaration.
pub(crate) fn pascal(name: &str) -> String {
    Symbol::Declared(name).pascal()
}

/// Returns the C# property name for `field` on `owner`.
pub(crate) fn property(field: &Field, owner: &str) -> String {
    let name = Symbol::Field(field).name().pascal();

    if name == owner || RESERVED_MEMBERS.contains(&name.as_str()) {
        name + PROPERTY_SUFFIX
    } else {
        name
    }
}

/// Returns `field` as an escaped camelCase parameter or local identifier.
pub(crate) fn parameter(field: &Field) -> String {
    escape(&Symbol::Field(field).name().camel())
}

pub(crate) fn escape(name: &str) -> String {
    if KEYWORDS.contains(&name) {
        format!("@{name}")
    } else {
        name.to_owned()
    }
}

/// Returns C# type text for `kind`.
pub(crate) fn number(kind: NumberKind) -> &'static str {
    match kind {
        NumberKind::U8 => "byte",
        NumberKind::I8 => "sbyte",
        NumberKind::U16 => "ushort",
        NumberKind::I16 => "short",
        NumberKind::U32 | NumberKind::Fixed32 => "uint",
        NumberKind::I32 => "int",
        NumberKind::U64 | NumberKind::Fixed64 => "ulong",
        NumberKind::I64 => "long",
        NumberKind::F32 => "float",
        NumberKind::F64 => "double",
    }
}

/// Returns the `DuplexWriter` and `DuplexReader` numeric method suffix, such as `U8` in `WriteU8` and `ReadU8`.
pub(crate) fn number_name(kind: NumberKind) -> String {
    let keyword = kind.keyword();
    let mut chars = keyword.chars();

    chars.next().map_or_else(String::new, |first| {
        first.to_ascii_uppercase().to_string() + chars.as_str()
    })
}

/// Returns whether `encoding` uses a C# value type.
pub(crate) fn is_value_type(encoding: &Encoding<'_>) -> bool {
    matches!(
        encoding,
        Encoding::Bool | Encoding::Char | Encoding::Number(..) | Encoding::Enumeration(_)
    )
}

/// Returns C# type text for `encoding`.
pub(crate) fn of(encoding: &Encoding<'_>) -> String {
    match encoding {
        Encoding::Bool => "bool".to_owned(),
        Encoding::Char => "int".to_owned(),
        Encoding::String(_) => "string".to_owned(),
        Encoding::Bytes(_) => "byte[]".to_owned(),
        Encoding::Number(kind, _) => number(*kind).to_owned(),
        Encoding::List(item, _) => format!(
            "global::System.Collections.Generic.IReadOnlyList<{}>",
            of(item)
        ),
        Encoding::Record(item, _) => format!(
            "global::System.Collections.Generic.IReadOnlyDictionary<string, {}>",
            of(item)
        ),
        Encoding::Nullable(inner) => format!("{}?", of(inner)),
        Encoding::Struct(name) | Encoding::Union(name) | Encoding::Enumeration(name) => {
            pascal(name)
        }
    }
}

/// Converts `value` to a C# expression using `encoding`.
pub(crate) fn literal(value: &Value, encoding: &Encoding<'_>) -> String {
    match (value, encoding.non_null()) {
        (Value::Null, _) => "null".to_owned(),
        (Value::Bool(value), _) => value.to_string(),
        (Value::Char(value), _) => u32::from(*value).to_string(),
        (Value::Text(text), _) => string(text),
        (Value::Member { name: member, .. }, Encoding::Enumeration(name)) => {
            format!("{}.{}", pascal(name), pascal(member))
        }
        (Value::Member { value, .. }, _) => value.to_string(),
        (Value::Integer(value), Encoding::Number(kind, _)) => integer(*value, *kind),
        (Value::Integer(value), _) => value.to_string(),
        (Value::Float(value), Encoding::Number(NumberKind::F32, _)) => {
            format!("{}f", float(*value))
        }
        (Value::Float(value), _) => float(*value),
    }
}

/// Converts `value` to a C# expression as `kind`.
pub(crate) fn integer(value: i128, kind: NumberKind) -> String {
    match kind {
        NumberKind::U32 | NumberKind::Fixed32 => format!("{value}u"),
        NumberKind::U64 | NumberKind::Fixed64 => format!("{value}ul"),
        NumberKind::I64 if value == i128::from(i64::MIN) => "long.MinValue".to_owned(),
        NumberKind::I64 => format!("{value}L"),
        NumberKind::I32 if value == i128::from(i32::MIN) => "int.MinValue".to_owned(),
        NumberKind::F32 => format!("{value}f"),
        NumberKind::F64 => format!("{value}d"),
        _ => value.to_string(),
    }
}

pub(crate) fn float(value: f64) -> String {
    if value.is_infinite() {
        return if value > 0.0 {
            "double.PositiveInfinity"
        } else {
            "double.NegativeInfinity"
        }
        .to_owned();
    }

    let text = format!("{value:?}");

    if text.contains(['.', 'e', 'E']) {
        text
    } else {
        format!("{text}.0")
    }
}

/// Returns `text` as an escaped C# string literal.
pub(crate) fn string(text: &str) -> String {
    let mut literal = String::from("\"");

    for char in text.chars() {
        match char {
            '"' => literal.push_str("\\\""),
            '\\' => literal.push_str("\\\\"),
            '\n' => literal.push_str("\\n"),
            '\r' => literal.push_str("\\r"),
            '\t' => literal.push_str("\\t"),
            char if char.is_control() => {
                let _ = write!(literal, "\\u{:04x}", u32::from(char));
            }
            char => literal.push(char),
        }
    }

    literal.push('"');

    literal
}
