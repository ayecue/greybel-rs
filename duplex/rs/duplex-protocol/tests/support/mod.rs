pub mod folders;
pub mod plan;
pub mod tags;
pub mod versions;

use std::fmt::Display;

use duplex_protocol::{
    Definition, Field, Plan, Protocol, TypeKind, Version, VersionHistory, VersionsError,
};

pub const SHOP: &str = r#"
use core 1.0;

/// Orders between a shop and its customers.
protocol Shop = "shop" {
    /// The shop.
    provider Counter {
        /// Something to buy.
        message Order {
            item: string,
            count: u32,
            gift?: bool,
            note: string?,
        } -> Result<Receipt, OutOfStock>
        message Cancel {}
    }

    client Customer {
        message Confirm {
            flatten price: Price,
        } -> bool
    }
}

/// What an order cost.
union Receipt {
    Paid { total: f64, items: list<string> },
    Declined(string),
    Waiting,
}

enum OutOfStock {
    Sold,
    Delisted,
}

struct Price {
    amount: i64,
    taxed: bool,
}

enum Size {
    Small,
    /// Neither small nor large.
    Medium,
    Large,
}

struct Shelf {
    sizes: record<Size>,
    letters: list<char?>,
}
"#;

/// Parses `source`, panicking with its validation error on failure.
pub fn definition(source: &str) -> Definition {
    Definition::parse(source)
        .unwrap_or_else(|error| panic!("the definition doesn't parse: {error}"))
}

pub fn protocol(definition: &Definition) -> &Protocol {
    definition
        .protocol()
        .expect("the definition declares a protocol")
}

/// Returns the fields of struct `name`, panicking if the declaration is missing or is not a struct.
pub fn struct_fields<'d>(definition: &'d Definition, name: &str) -> &'d [Field] {
    let declaration = definition
        .declaration(name)
        .unwrap_or_else(|| panic!("the definition declares `{name}`"));

    match &declaration.kind {
        TypeKind::Struct(fields) => fields,
        kind => panic!("`{name}` is a struct, not {kind:?}"),
    }
}

/// Builds a core-1.0 protocol definition named `P` from `body` and trailing type declarations.
pub fn protocol_source(body: &str, types: &str) -> String {
    format!("use core 1.0; protocol P = \"p\" {{ {body} }}\n{types}")
}

/// Builds a `test` version history from `(major, minor, source)` tuples.
pub fn version_history(sources: &[(u16, u16, &str)]) -> Result<VersionHistory, VersionsError> {
    let sources: Vec<(Version, &str)> = sources
        .iter()
        .map(|&(major, minor, source)| (Version::new(major, minor), source))
        .collect();

    VersionHistory::of_sources("test", &sources)
}

pub fn plan(versions: &VersionHistory) -> Plan<'_> {
    Plan::of(versions.major(1).expect("the versions have major 1"))
}

/// Returns the error text from `result`, panicking if it is `Ok`.
pub fn error_message<T>(result: Result<T, impl Display>) -> String {
    match result {
        Ok(_) => panic!("the source is accepted instead of rejected"),
        Err(error) => error.to_string(),
    }
}
