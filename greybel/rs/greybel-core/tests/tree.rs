use greybel_core::NodeKind;

use crate::support::tree::path_names;

#[test]
fn should_descend_to_the_deepest_node_at_an_offset() {
    let source = "x = foo.bar(1, \"a\")\nif x then\n  y = [x]\nend if";
    let ast = greybel_core::parse(source, greybel_core::ParserOptions::default()).unwrap();
    let at = |text: &str| source.find(text).unwrap();
    let offsets = [
        at("bar"),
        at("bar") + 3,
        at("(1") + 1,
        at("1"),
        at("[x]") + 1,
        source.len() + 10,
    ];

    let [bar, after_bar, argument, one, list_value, past_end] =
        offsets.map(|offset| path_names(&ast, offset));

    assert_eq!(
        bar,
        [
            "Chunk",
            "AssignmentStatement",
            "CallExpression",
            "MemberExpression",
            "Identifier"
        ]
    );
    assert_eq!(
        after_bar,
        [
            "Chunk",
            "AssignmentStatement",
            "CallExpression",
            "MemberExpression",
            "Identifier"
        ]
    );
    assert_eq!(
        argument,
        [
            "Chunk",
            "AssignmentStatement",
            "CallExpression",
            "NumericLiteral"
        ]
    );
    assert_eq!(
        one,
        [
            "Chunk",
            "AssignmentStatement",
            "CallExpression",
            "NumericLiteral"
        ]
    );
    assert_eq!(
        list_value,
        [
            "Chunk",
            "IfStatement",
            "IfClause",
            "AssignmentStatement",
            "ListConstructorExpression",
            "ListValue",
            "Identifier"
        ]
    );
    assert_eq!(past_end, ["Chunk"]);
}

#[test]
fn should_take_literal_text_from_the_source_under_its_span() {
    let ast = greybel_core::parse(
        "x = [\"a\"\"b\", 1.50, -2, true, null]",
        greybel_core::ParserOptions::default(),
    )
    .unwrap();

    let texts: Vec<&str> = ast
        .literals
        .iter()
        .map(|&id| ast.text(id).unwrap())
        .collect();

    for &id in &ast.literals {
        assert!(matches!(ast.kind(id), NodeKind::Literal(_)));
    }

    assert_eq!(texts, ["\"a\"\"b\"", "1.50", "2", "true", "null"]);
}
