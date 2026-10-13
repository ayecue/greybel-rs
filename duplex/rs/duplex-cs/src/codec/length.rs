use duplex_protocol::Length;

use super::{READER, WRITER};

pub(super) fn length_arguments(length: Length) -> String {
    let max = length
        .max
        .map_or_else(|| "long.MaxValue".to_owned(), |max| format!("{max}L"));

    format!("{}L, {max}", length.min)
}

pub(super) fn write_length(length: &str, requirement: Length) -> String {
    if requirement.is_any() {
        format!("{WRITER}.WriteLength({length});")
    } else {
        format!(
            "{WRITER}.WriteValidatedLength({length}, {});",
            length_arguments(requirement)
        )
    }
}

pub(super) fn read_length(requirement: Length) -> String {
    if requirement.is_any() {
        format!("{READER}.ReadLength()")
    } else {
        format!(
            "{READER}.ReadValidatedLength({})",
            length_arguments(requirement)
        )
    }
}
