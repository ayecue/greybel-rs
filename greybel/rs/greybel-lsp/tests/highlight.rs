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

#[test]
fn should_classify_the_code_inside_a_string_template() {
    let mut client = Client::new(&[]);
    let initialized = client.initialize();
    let Some(lsp::SemanticTokensProvider::SemanticTokensOptions(options)) =
        initialized.capabilities.semantic_tokens_provider
    else {
        panic!("semantic tokens");
    };

    client.open("main.src", "n = 1\nx = #\"a{n + 2}b\"\n");

    let tokens = client
        .request::<lsp::SemanticTokensRequest>(
            json!({ "textDocument": { "uri": uri("main.src") } }),
        )
        .expect("tokens");
    let tokens = absolute(&tokens.data, &options.legend.token_types);
    let line: Vec<(u32, u32, &str)> = tokens
        .iter()
        .filter(|token| token.0 == 1 && token.1 >= 4)
        .map(|token| (token.1, token.2, token.3.as_str()))
        .collect();

    assert_eq!(
        line,
        [
            (4, 4, "string"),
            (8, 1, "variable"),
            (10, 1, "operator"),
            (12, 1, "number"),
            (13, 3, "string"),
        ]
    );
}

#[test]
fn should_classify_contextual_keywords_and_types() {
    let mut client = Client::new(&[]);
    let initialized = client.initialize();
    let Some(lsp::SemanticTokensProvider::SemanticTokensOptions(options)) =
        initialized.capabilities.semantic_tokens_provider
    else {
        panic!("semantic tokens");
    };

    client.open(
        "main.src",
        "let p: Port = 1\n{name, port as q} = {}\nx = p as number\ntype Port = number\ntype Repo extends map\n  readonly id: Port\n  find(key: string): map\nend type\nas = 2\n",
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
            .map(|token| token.3.as_str())
    };

    assert_eq!(at(0, 0), Some("keyword"));
    assert_eq!(at(0, 7), Some("type"));
    assert_eq!(at(1, 12), Some("keyword"));
    assert_eq!(at(2, 6), Some("keyword"));
    assert_eq!(at(2, 9), Some("type"));
    assert_eq!(at(3, 0), Some("keyword"));
    assert_eq!(at(3, 5), Some("type"));
    assert_eq!(at(4, 10), Some("keyword"));
    assert_eq!(at(4, 18), Some("type"));
    assert_eq!(at(5, 2), Some("keyword"));
    assert_eq!(at(5, 11), Some("property"));
    assert_eq!(at(5, 15), Some("type"));
    assert_eq!(at(6, 2), Some("function"));
    assert_eq!(at(6, 7), Some("parameter"));
    assert_eq!(at(6, 12), Some("type"));
    assert_eq!(at(8, 0), Some("variable"));
}

#[test]
fn should_mark_api_function_names_as_default_library() {
    let mut client = Client::new(&[]);
    let initialized = client.initialize();
    let Some(lsp::SemanticTokensProvider::SemanticTokensOptions(options)) =
        initialized.capabilities.semantic_tokens_provider
    else {
        panic!("semantic tokens");
    };

    assert_eq!(options.legend.token_modifiers, ["defaultLibrary"]);

    client.open(
        "main.src",
        "f = function(x)\n  return x\nend function\nprint(f([1].len))\nshell = get_shell\nshell.host_computer\n",
    );

    let tokens = client
        .request::<lsp::SemanticTokensRequest>(
            json!({ "textDocument": { "uri": uri("main.src") } }),
        )
        .expect("tokens");
    let (mut line, mut character) = (0, 0);
    let mut library = Vec::new();

    for token in &tokens.data {
        line += token.delta_line;
        character = if token.delta_line == 0 {
            character + token.delta_start
        } else {
            token.delta_start
        };

        if token.token_modifiers_bitset == 1 {
            library.push((line, character));
        }
    }

    assert_eq!(library, [(3, 0), (4, 8), (5, 6)]);
}
