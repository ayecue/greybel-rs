#[test]
fn should_lex_non_ascii_identifiers_without_errors() {
    let sources = ["é", "日本 = 1", "x = é + 本", "😀"];

    let outputs = sources.map(|source| {
        (
            source,
            greybel_core::parse_unsafe(source, greybel_core::ParserOptions::default()),
        )
    });

    for (source, output) in outputs {
        assert!(output.lexer_errors.is_empty(), "{source}");
    }
}

#[test]
fn should_keep_non_ascii_identifiers_in_the_tree() {
    let source = "日本 = 1";

    let ast = greybel_core::parse(source, greybel_core::ParserOptions::default()).unwrap();

    assert!(ast.to_string().contains("日本"));
}
