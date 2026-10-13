//! Tests semantic tokens from lexical syntax and parser-resolved identifiers.

use greybel_lsp::types as lsp;
use serde_json::json;

use crate::support::highlight::absolute;
use crate::support::{Client, uri};

#[test]
fn should_classify_tokens_and_identifiers() {
    let mut client = Client::new(&[]);
    let initialized = client.initialize();
    let Some(lsp::SemanticTokensProvider::SemanticTokensOptions(options)) =
        initialized.capabilities.semantic_tokens_provider
    else {
        panic!("semantic tokens");
    };

    client.open(
        "main.src",
        "// hi\nf = function(a, b = 2)\n  return a.len + \"x\"\nend function\nf(true)\nprint null\n",
    );

    let tokens = client
        .request::<lsp::SemanticTokensRequest>(
            json!({ "textDocument": { "uri": uri("main.src") } }),
        )
        .expect("tokens");
    let tokens = absolute(&tokens.data, &options.legend.token_types);
    let at = |line: u32, character: u32| {
        tokens
            .iter()
            .find(|token| token.0 == line && token.1 == character)
            .map(|token| (token.2, token.3.as_str()))
    };

    assert_eq!(at(0, 0), Some((5, "comment")));
    assert_eq!(at(1, 0), Some((1, "variable")));
    assert_eq!(at(1, 2), Some((1, "operator")));
    assert_eq!(at(1, 4), Some((8, "keyword")));
    assert_eq!(at(1, 12), Some((1, "punctuator")));
    assert_eq!(at(1, 13), Some((1, "parameter")));
    assert_eq!(at(1, 20), Some((1, "number")));
    assert_eq!(at(2, 11), Some((3, "property")));
    assert_eq!(at(2, 17), Some((3, "string")));
    assert_eq!(at(4, 0), Some((1, "function")));
    assert_eq!(at(4, 2), Some((4, "constant")));
    assert_eq!(at(5, 0), Some((5, "function")));
    assert_eq!(at(5, 6), Some((4, "constant")));
}

#[test]
fn should_split_a_token_spanning_lines() {
    let mut client = Client::new(&[]);

    client.initialize();
    client.open("main.src", "x = 1 /* one\ntwo */\n");

    let tokens = client
        .request::<lsp::SemanticTokensRequest>(
            json!({ "textDocument": { "uri": uri("main.src") } }),
        )
        .expect("tokens");
    let comments: Vec<_> = tokens
        .data
        .iter()
        .filter(|token| token.token_type == 8)
        .collect();

    assert_eq!(comments.len(), 2, "{tokens:?}");
    assert_eq!(
        (
            comments[0].length,
            comments[1].delta_line,
            comments[1].length
        ),
        (6, 1, 6)
    );
}
