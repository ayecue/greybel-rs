use duplex_protocol::{Definition, Encoding, NumberKind, Value};

use crate::types;

/// Returns numeric minimum and maximum C# arguments for the matching `ValidateRange` overload, or `None` when
/// unbounded.
pub(super) fn bounds_of(kind: NumberKind, encoding: &Encoding<'_>) -> Option<(String, String)> {
    let Encoding::Number(_, bounds) = encoding else {
        return None;
    };

    if bounds.is_any() {
        return None;
    }

    let (min, max) = Definition::bounds_of(kind, bounds).expect("the constraints verify bounds");
    let csharp = types::number(kind);
    let bound = |value: Option<Value>, end: &str| match (value, range_type(kind)) {
        (Some(Value::Integer(value)), "ulong") => format!("{value}ul"),
        (Some(Value::Integer(value)), "long") => format!("{value}L"),
        (Some(Value::Integer(value)), _) => format!("{value}d"),
        (Some(Value::Float(value)), _) => types::float(value),
        (_, "double") => format!(
            "double.{}Infinity",
            if end == "Min" { "Negative" } else { "Positive" }
        ),
        (_, range) => format!("({range}){csharp}.{end}Value"),
    };

    Some((bound(min, "Min"), bound(max, "Max")))
}

/// Returns the `ValidateRange` overload parameter type for `kind`.
pub(super) fn range_type(kind: NumberKind) -> &'static str {
    match kind {
        NumberKind::U64 | NumberKind::Fixed64 => "ulong",
        NumberKind::F32 | NumberKind::F64 => "double",
        _ => "long",
    }
}

/// Returns an expression validating `value` against `bounds` with `side` and casting the result to the C# type for `kind`.
pub(super) fn range_check(
    side: &str,
    kind: NumberKind,
    value: &str,
    bounds: &(String, String),
) -> String {
    let range = range_type(kind);
    let csharp = types::number(kind);
    let check = format!(
        "{side}.ValidateRange(({range}){value}, {}, {})",
        bounds.0, bounds.1
    );

    if csharp == range {
        check
    } else {
        format!("({csharp}){check}")
    }
}
