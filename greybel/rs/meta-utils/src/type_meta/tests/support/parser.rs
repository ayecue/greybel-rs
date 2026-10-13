use super::super::parse;

pub(super) fn parsed(text: &str) -> String {
    parse(text).map_or_else(|error| format!("error: {error}"), |meta| meta.to_string())
}

pub(super) fn nested(depth: usize) -> String {
    format!("{}number{}", "list<".repeat(depth), ">".repeat(depth))
}
