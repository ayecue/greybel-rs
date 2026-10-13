//! Shared helpers for `greybel-symbols` integration tests.

/// Maps each symbol through `text` and then `parse` for round-trip tests.
pub fn parsed_from_texts<S: Copy>(
    symbols: &[S],
    text: fn(S) -> &'static str,
    parse: fn(&str) -> Option<S>,
) -> Vec<Option<S>> {
    symbols.iter().map(|&symbol| parse(text(symbol))).collect()
}
