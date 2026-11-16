//! Tests `@ignore` and `@ignore-file` comments.

use greybel_type_analyzer::{DiagnosticKind, Dialect, DocComment};

use crate::support::diagnostics::diagnostics;
use crate::support::ignores::{KINDS, reported_lines};
use crate::support::{PATH, globals_in, with_document};

#[test]
fn should_ignore_the_next_line_with_code() {
    let source = "// @ignore\n\n// a note\nprint missing\nprint other";

    let reported = reported_lines(source);

    assert_eq!(reported, [(DiagnosticKind::UnresolvedReference, 5)]);
}

#[test]
fn should_ignore_its_own_line_after_code() {
    let source = "print missing // @ignore\nprint other";

    let reported = reported_lines(source);

    assert_eq!(reported, [(DiagnosticKind::UnresolvedReference, 2)]);
}

#[test]
fn should_ignore_only_the_named_codes() {
    let source = "// @ignore native-override, used-before-defined\nprint = missing";

    let reported = reported_lines(source);

    assert_eq!(reported, [(DiagnosticKind::UnresolvedReference, 2)]);
}

#[test]
fn should_ignore_the_whole_file() {
    let all = "print missing\n// @ignore-file\nprint = other";
    let named = "// @ignore-file unresolved-reference\nprint missing\nprint = 1";

    let all = reported_lines(all);
    let named = reported_lines(named);

    assert_eq!(all, []);
    assert_eq!(named, [(DiagnosticKind::NativeOverride, 3)]);
}

#[test]
fn should_report_a_code_that_names_no_diagnostic() {
    let source = "// @ignore no-such-code\nprint missing";
    let workspace = with_document(Dialect::MiniScript, source);

    let reported = diagnostics(&workspace, KINDS);

    assert_eq!(
        reported,
        [
            (
                DiagnosticKind::UnresolvedReference,
                2,
                "Cannot find \"missing\".".to_owned()
            ),
            (
                DiagnosticKind::InvalidAnnotation,
                1,
                "@ignore names no diagnostic \"no-such-code\".".to_owned()
            ),
        ]
    );
}

#[test]
fn should_leave_ignore_tags_out_of_a_doc_comment() {
    let lines = [
        " Adds one.",
        " @ignore unused-parameter",
        " @param {number} x",
    ];

    let doc = DocComment::parse(&lines);

    assert_eq!(doc.description.as_deref(), Some("Adds one."));
    assert_eq!(doc.params.len(), 1);
    assert_eq!(doc.invalid_tags, []);
}

#[test]
fn should_ignore_a_line_documented_by_the_same_comment() {
    let source = "// Adds one.\n// @ignore unused-parameter\n// @param {number} x\nf = function(x, unused)\n  return x + 1\nend function\nr = f(1)";
    let workspace = with_document(Dialect::MiniScript, source);

    let unused = diagnostics(&workspace, &[DiagnosticKind::UnusedParameter]);
    let types = globals_in(&workspace, PATH, &["r"]);

    assert_eq!(unused, []);
    assert_eq!(types, ["number"]);
}
