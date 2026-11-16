//! Tests analysis and IDE queries on parser-depth-limit code without stack overflow on native and wasm targets.

use std::fmt::Write as _;

use greybel_core::Position;
use greybel_test_support::node_depth::{MAX_ACCEPTED_DEPTH, programs_at_max_depth};
use greybel_type_analyzer::{Analyzer, DiagnosticKind, Dialect};

use crate::support::{PATH, main_global, uri, with_document};

#[test]
fn should_analyze_code_at_the_depth_limit() {
    let cases = programs_at_max_depth();

    let results: Vec<_> = cases
        .into_iter()
        .map(|(shape, source)| {
            let workspace = with_document(Dialect::Greybel, &source);
            let analyzed = Analyzer::new(&workspace).analysis(&uri(PATH)).is_some();
            let syntax = Analyzer::new(&workspace)
                .diagnostics(&uri(PATH))
                .into_iter()
                .filter(|diagnostic| diagnostic.kind == DiagnosticKind::Syntax)
                .count();
            let middle = Position::new(
                1 + u32::try_from(source.lines().count())
                    .expect("value fits the target integer type")
                    / 2,
                2,
            );
            let _ = Analyzer::new(&workspace).hover(&uri(PATH), middle);
            let _ = Analyzer::new(&workspace).completions(&uri(PATH), middle);
            let _ = Analyzer::new(&workspace).document_symbols(&uri(PATH));

            (shape, analyzed, syntax)
        })
        .collect();

    for (shape, analyzed, syntax) in results {
        assert!(analyzed, "{shape}");
        assert_eq!(syntax, 0, "{shape}");
    }
}

#[test]
fn should_fall_back_instead_of_overflowing_on_calls_into_functions_at_the_depth_limit() {
    let depth = MAX_ACCEPTED_DEPTH;
    let mut source = format!(
        "f0 = function(x)\n  return {}x{}\nend function\n",
        "(".repeat(depth - 2),
        ")".repeat(depth - 2)
    );

    for level in 1..12 {
        let _ = write!(
            source,
            "f{level} = function(x)\n  return {}f{}(x){}\nend function\n",
            "(".repeat(depth - 2),
            level - 1,
            ")".repeat(depth - 2)
        );
    }

    let _ = writeln!(
        source,
        "y = {}f11(1){}",
        "(".repeat(depth - 2),
        ")".repeat(depth - 2)
    );

    let workspace = with_document(Dialect::MiniScript, &source);

    let y = main_global(&workspace, "y");

    assert_eq!(y, "any");
}
