//! Tests analysis and IDE queries on parser-depth-limit code without stack overflow on native and wasm targets.

use std::fmt::Write as _;

use greybel_core::{Position, Range};
use greybel_test_support::node_depth::{
    CHAIN_LENGTH, MAX_ACCEPTED_DEPTH, operator_chains, programs_at_max_depth,
};
use greybel_type_analyzer::{Analyzer, DiagnosticKind, Dialect, InlayHintOptions};

use crate::support::{PATH, main_global, uri, with_document};

#[test]
fn should_analyze_operator_chains_of_any_length() {
    let cases = operator_chains(CHAIN_LENGTH);

    let results: Vec<_> = cases
        .into_iter()
        .map(|(shape, source)| {
            let workspace = with_document(Dialect::Greybel, &source);
            let analyzer = Analyzer::new(&workspace);
            let analyzed = analyzer.analysis(&uri(PATH)).is_some();
            let diagnostics: Vec<String> = analyzer
                .diagnostics(&uri(PATH))
                .into_iter()
                .map(|diagnostic| diagnostic.message)
                .collect();
            let last = u32::try_from(source.lines().count() - 1).expect("lines fit u32");
            let middle = Position::new(last, 4_000);
            let whole = Range::new(Position::new(0, 0), Position::new(last + 1, 0));
            let _ = analyzer.hover(&uri(PATH), middle);
            let _ = analyzer.completions(&uri(PATH), middle);
            let _ = analyzer.signature_help(&uri(PATH), middle);
            let _ = analyzer.references(&uri(PATH), middle);
            let _ = analyzer.selection_ranges(&uri(PATH), middle);
            let _ = analyzer.code_actions(&uri(PATH), whole);
            let _ = analyzer.inlay_hints(&uri(PATH), whole, InlayHintOptions::default());
            let _ = analyzer.document_symbols(&uri(PATH));
            let _ = analyzer.definition(&uri(PATH), middle);
            let _ = analyzer.folds(&uri(PATH));
            let _ = analyzer.document_colors(&uri(PATH));
            let _ = analyzer.outgoing_calls(&uri(PATH), middle);
            let _ = analyzer.incoming_calls(&uri(PATH), Position::new(0, 0));
            let _ = analyzer.rename(&uri(PATH), Position::new(0, 0), "g");

            (shape, analyzed, diagnostics)
        })
        .collect();

    for (shape, analyzed, diagnostics) in results {
        assert!(analyzed, "{shape}");
        assert!(diagnostics.is_empty(), "{shape}: {diagnostics:?}");
    }
}

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
