use std::collections::BTreeMap;

use greybel_core::{Ast, ColumnSettings, Dialect, NodeId, ParserOptions, Version};
use greybel_symbols::names::UNNAMED_SOURCE;
use serde::Serialize;
use serde_json::Value;
use support::outputs::{Setup, columns, lexer, parser};

use crate::support;

#[derive(Serialize)]
pub struct Case {
    pub input: Value,
    pub output: Value,
}

/// TypeScript fixture setup: MiniScript 1.6 with no file name or environment.
pub const MINISCRIPT: Setup = Setup {
    name: "miniscript",
    options: |_| ParserOptions {
        version: Version::V1_6,
        ..ParserOptions::new(Dialect::MiniScript)
    },
    extras: |_, _| BTreeMap::new(),
};

pub const GREYBEL: Setup = Setup {
    name: "greybel",
    options: |case| case_options(Dialect::Greybel, case),
    extras: greybel_extras,
};

pub const GREYSCRIPT: Setup = Setup {
    name: "greyscript",
    options: |case| case_options(Dialect::GreyScript, case),
    extras: greyscript_extras,
};

/// Builds parser options for `dialect` using the fixture's file name and environment.
pub fn case_options(dialect: Dialect, case: &Value) -> ParserOptions {
    let environment = case["env"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(name, value)| {
            let value = value.as_str().unwrap_or_default().to_owned();

            (name.clone(), value)
        })
        .collect();

    ParserOptions {
        filename: case["filename"]
            .as_str()
            .unwrap_or(UNNAMED_SOURCE)
            .to_owned(),
        environment,
        ..ParserOptions::new(dialect)
    }
}

pub fn dump(ast: &Ast, ids: &[NodeId], columns: ColumnSettings) -> Vec<String> {
    ids.iter().map(|&id| ast.dump(id, columns)).collect()
}

pub fn greybel_extras(ast: &Ast, columns: ColumnSettings) -> BTreeMap<String, Vec<String>> {
    BTreeMap::from([
        ("imports".to_owned(), dump(ast, &ast.imports, columns)),
        ("includes".to_owned(), dump(ast, &ast.includes, columns)),
        ("injects".to_owned(), dump(ast, &ast.injects, columns)),
    ])
}

pub fn greyscript_extras(ast: &Ast, columns: ColumnSettings) -> BTreeMap<String, Vec<String>> {
    let mut extras = greybel_extras(ast, columns);

    extras.insert(
        "nativeImports".to_owned(),
        dump(ast, &ast.native_imports, columns),
    );

    extras
}

pub fn inputs(dialect: &Setup) -> Vec<Value> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(dialect.name)
        .join("inputs.json");
    let text = std::fs::read_to_string(&path).expect("inputs readable");

    serde_json::from_str(&text).expect("inputs are valid JSON")
}

/// Returns lexer and parser cases for the dialect's inputs, with lexer cases first.
pub fn outputs(dialect: &Setup) -> (Vec<Case>, Vec<Case>) {
    inputs(dialect)
        .into_iter()
        .map(|input| {
            let output = if input["kind"] == "lexer" {
                let content = input["content"].as_str().expect("content");
                let grammar = (dialect.options)(&input).grammar();
                let unsafe_mode = input["unsafe"].as_bool().unwrap_or(false);

                lexer(content, grammar, columns(&input), unsafe_mode)
            } else {
                parser(&input, dialect)
            };

            Case { input, output }
        })
        .partition(|case| case.input["kind"] == "lexer")
}
