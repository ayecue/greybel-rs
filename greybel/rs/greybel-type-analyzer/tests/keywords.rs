//! Tests keyword, snippet and doc-comment completions.

use crate::support::keywords::{keyword_labels, labels_at, snippet};

#[test]
fn should_complete_statement_keywords_at_the_start_of_a_line() {
    let labels = keyword_labels("f = function(x)\n  while x\n    |\n  end while\nend function");

    assert_eq!(
        labels,
        [
            "if",
            "if else",
            "for",
            "while",
            "function",
            "return",
            "else",
            "else if",
            "break",
            "continue",
            "end while"
        ]
    );
}

#[test]
fn should_leave_loop_keywords_out_of_functions_in_loops() {
    let labels = keyword_labels("for i in [1]\n  f = function\n    |\n  end function\nend for");

    assert!(!labels.contains(&"break".to_owned()), "{labels:?}");
    assert!(labels.contains(&"end function".to_owned()), "{labels:?}");
}

#[test]
fn should_complete_after_a_multibyte_character() {
    let after_symbol = keyword_labels("x = 1 ¶ |");
    let after_name = keyword_labels("x = ¶a |");
    let in_comment = labels_at("// @¶par|\nf = function\nend function");

    assert_eq!(
        after_symbol,
        ["function", "true", "false", "null", "new", "not"]
    );
    assert_eq!(after_name, ["and", "or", "isa"]);
    assert!(in_comment.is_empty(), "{in_comment:?}");
}

#[test]
fn should_complete_values_in_an_expression() {
    let labels = keyword_labels("x = |");

    assert_eq!(labels, ["function", "true", "false", "null", "new", "not"]);
}

#[test]
fn should_complete_operators_after_an_operand() {
    assert_eq!(keyword_labels("x = a |"), ["and", "or", "isa"]);
    assert_eq!(keyword_labels("x = f(1) |"), ["and", "or", "isa"]);
    assert_eq!(
        keyword_labels("x = not |"),
        ["function", "true", "false", "null", "new", "not"]
    );
}

#[test]
fn should_complete_statements_after_then() {
    let labels = keyword_labels("if x then |");

    assert!(labels.contains(&"return".to_owned()), "{labels:?}");
}

#[test]
fn should_insert_block_snippets_with_tab_stops() {
    assert_eq!(
        snippet("|", "for").as_deref(),
        Some("for ${1:item} in ${2:list}\n\t$0\nend for")
    );
}

#[test]
fn should_complete_tags_types_and_parameter_names_in_doc_comments() {
    let tags = labels_at("// @pa|\nf = function(host, port)\nend function");
    let types =
        labels_at("// @type Point\n// @param {str|} host\nf = function(host, port)\nend function");
    let parameters = labels_at("// @param {string} |\nf = function(host, port = 22)\nend function");
    let text = labels_at("// Connects to a |\nf = function(host)\nend function");

    assert!(
        tags.contains(&"param".to_owned()) && tags.contains(&"deprecated".to_owned()),
        "{tags:?}"
    );
    assert!(
        types.contains(&"string".to_owned()) && types.contains(&"Point".to_owned()),
        "{types:?}"
    );
    assert!(types.contains(&"shell".to_owned()), "{types:?}");
    assert_eq!(parameters, ["host", "port"]);
    assert_eq!(text, Vec::<String>::new());
}
