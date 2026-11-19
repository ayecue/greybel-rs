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
        (
            "lists",
            format!("x = {}{}", "[".repeat(depth), "]".repeat(depth)),
        ),
        (
            "maps",
            format!("x = {}1{}", "{\"a\": ".repeat(depth), "}".repeat(depth)),
        ),
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

/// Operator chain length the passes are tested with: past any recursion limit, as left-associative operators chain
/// without nesting.
pub const CHAIN_LENGTH: usize = 20_000;

/// Creates `(shape, source)` programs of `length` operations chained by left-associative operators: no nesting
/// limit applies to them, as in Grey Hack.
#[must_use]
pub fn operator_chains(length: usize) -> Vec<(&'static str, String)> {
    vec![
        ("additions", format!("x = 1{}", " + 1".repeat(length))),
        (
            "concatenations",
            format!("x = \"a\"{}", " + char(10) + \"b\"".repeat(length / 2)),
        ),
        (
            "arithmetic",
            format!("x = 1{}", " + 2 * 3 - 4 / 2 % 3".repeat(length / 5)),
        ),
        ("powers", format!("x = 1{}", " ^ 1".repeat(length))),
        (
            "conditions",
            format!("x = 1{}", " and 1 or 0".repeat(length / 2)),
        ),
        (
            "if conditions",
            format!(
                "a = 1\nif a == 0{} then\n  print(a)\nend if\nwhile a{}\n  a = 0\nend while",
                " or a == 1 and a".repeat(length / 2),
                " and a".repeat(length),
            ),
        ),
        (
            "function bodies",
            format!(
                "f = function(a)\n  return a{}\nend function\nf(1)",
                " + a".repeat(length)
            ),
        ),
        (
            "value expressions",
            format!(
                "a = null\nx = (a ?? 1){0}\ny = 1{0} + (a ?? 2)\nz = (a?.len ?? 0){0}",
                " + 1".repeat(length),
            ),
        ),
        (
            "calls",
            format!(
                "f = function(a)\n  return a\nend function\nx = f(1){}",
                " + f(1)".repeat(length)
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
