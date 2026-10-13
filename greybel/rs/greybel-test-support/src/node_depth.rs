//! Programs near the parser's `MAX_NODE_DEPTH` and assertions on their processing outcomes.

use std::fmt::Display;

use greybel_core::MAX_NODE_DEPTH;

/// Creates `(shape, source)` programs for each nesting form at `depth`. Statement, assignment, and literal wrappers
/// add about eight parser levels.
#[must_use]
pub fn nested_programs(depth: usize) -> Vec<(&'static str, String)> {
    vec![
        (
            "parentheses",
            format!("x = {}1{}", "(".repeat(depth), ")".repeat(depth)),
        ),
        ("powers", format!("x = 2{}", " ^ 2".repeat(depth))),
        (
            "lists",
            format!("x = {}{}", "[".repeat(depth), "]".repeat(depth)),
        ),
        (
            "maps",
            format!("x = {}1{}", "{\"a\": ".repeat(depth), "}".repeat(depth)),
        ),
        ("additions", format!("x = 1{}", " + 1".repeat(depth))),
        ("members", format!("a = {{}}\nx = a{}", ".b".repeat(depth))),
        ("indexes", format!("a = {{}}\nx = a{}", "[0]".repeat(depth))),
        (
            "calls",
            format!(
                "f = @print\nx = {}{}",
                "f(".repeat(depth),
                ")".repeat(depth)
            ),
        ),
        (
            "ifs",
            format!(
                "{}x = 1\n{}",
                "if 1 then\n".repeat(depth),
                "end if\n".repeat(depth)
            ),
        ),
        (
            "functions",
            format!(
                "{}x = 1\n{}",
                "f = function\n".repeat(depth),
                "end function\n".repeat(depth)
            ),
        ),
    ]
}

/// `MAX_NODE_DEPTH - 8`, reserving depth for [`nested_programs`] statement, assignment, and literal wrappers.
pub const MAX_ACCEPTED_DEPTH: usize = MAX_NODE_DEPTH - 8;

/// Creates [`nested_programs`] at [`MAX_ACCEPTED_DEPTH`].
#[must_use]
pub fn programs_at_max_depth() -> Vec<(&'static str, String)> {
    nested_programs(MAX_ACCEPTED_DEPTH)
}

/// Asserts `Ok` for every `(shape, result)` outcome.
///
/// # Panics
/// Panics with a `shape: error` line for every failure.
pub fn assert_no_failures<T, E: Display>(
    outcomes: impl IntoIterator<Item = (impl Display, Result<T, E>)>,
) {
    let failures: Vec<String> = outcomes
        .into_iter()
        .filter_map(|(shape, outcome)| outcome.err().map(|error| format!("{shape}: {error}")))
        .collect();

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
