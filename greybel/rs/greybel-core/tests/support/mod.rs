//! Shared helpers for `greybel-core` integration tests.

#![allow(dead_code)]

pub mod atoms;
pub mod cases;
pub mod fuzz;
pub mod keyword;
pub mod lexer_stream;
pub mod outputs;
pub mod parser_stream;
pub mod reported_line;
pub mod scopes;
pub mod snapshot;
pub mod strict;
pub mod token;
pub mod tree;

use greybel_core::{Dialect, Parser, ParserOptions, Version};

use crate::support::snapshot::render_strict;

/// Parses `source` and returns its AST text or the parse error message.
pub fn parse(source: &str) -> Result<String, String> {
    Parser::new(source, ParserOptions::default())
        .parse()
        .map(|ast| ast.to_string())
        .map_err(|error| error.message())
}

pub fn miniscript_options(version: Version) -> ParserOptions {
    ParserOptions {
        version,
        ..ParserOptions::new(Dialect::MiniScript)
    }
}

pub fn greybel_options() -> ParserOptions {
    ParserOptions::new(Dialect::Greybel)
}

pub fn rule_cases(source: &str) -> Vec<&str> {
    source.trim_end().split("\n---\n").collect()
}

pub fn render_rule_case(case: &str) -> String {
    let v1_6 = render_strict(case, &miniscript_options(Version::V1_6));
    let latest = render_strict(case, &miniscript_options(Version::Latest));
    let mut out = format!("=== {case:?}\n");

    if v1_6 == latest {
        out.push_str(&format!("1.6, latest: {v1_6}"));
    } else {
        out.push_str(&format!("1.6: {v1_6}latest: {latest}"));
    }

    out
}
