use duplex_protocol::VersionHistory;

use crate::support::struct_fields;

pub fn field_since(versions: &VersionHistory, major: u16, name: &str) -> Vec<u16> {
    let latest = versions
        .major(major)
        .expect("the versions have the requested major")
        .latest();

    struct_fields(&latest.definition, name)
        .iter()
        .map(|field| latest.additions.field(field))
        .collect()
}

pub const PROTOCOL: &str = "use core 1.0;\nprotocol P = \"p\" { client C { message Note { point: Point } } provider D {} }\n";
