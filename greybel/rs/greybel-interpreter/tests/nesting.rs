//! Tests that code at the parser's depth limit compiles without stack overflow on native and wasm targets.

use greybel_core::{Dialect, ParserOptions};
use greybel_documents::{DocumentOptions, Uri, Workspace};
use greybel_interpreter::{CompileOptions, Program};
use greybel_test_support::node_depth::{
    CHAIN_LENGTH, assert_no_failures, operator_chains, programs_at_max_depth,
};

use crate::support::cases::printed;

fn compile(programs: &[(&'static str, String)]) -> Vec<(&'static str, Result<(), String>)> {
    programs
        .iter()
        .map(|(shape, source)| {
            let mut workspace = Workspace::new(DocumentOptions {
                parser: ParserOptions::new(Dialect::Greybel),
                ..DocumentOptions::default()
            });
            let entry = Uri::memory("/main.src");

            workspace.set_file(&entry, source.as_str());

            let compiled = workspace
                .module_graph(&entry)
                .map_err(|error| error.to_string())
                .and_then(|graph| {
                    let options = CompileOptions {
                        dialect: Dialect::Greybel,
                        ..CompileOptions::default()
                    };

                    Program::compile(&graph, &options)
                        .map(drop)
                        .map_err(|error| error.to_string())
                });

            (*shape, compiled)
        })
        .collect()
}

#[test]
fn should_compile_code_at_the_depth_limit() {
    assert_no_failures(compile(&programs_at_max_depth()));
}

#[test]
fn should_compile_operator_chains_of_any_length() {
    assert_no_failures(compile(&operator_chains(CHAIN_LENGTH)));
}

#[test]
fn should_run_an_operator_chain_of_any_length() {
    let length = CHAIN_LENGTH;
    let code = format!(
        "x = 0{}\ny = 1{}\nprint(x)\nprint(y.len)\nprint(1 and 0 or 2 and 3)\nprint(2 * 3 + 10 / 4 - 1 % 2)",
        " + 1".repeat(length),
        " + \"c\" + \"ab\"".repeat(length),
    );

    assert_eq!(
        printed(&code),
        [
            length.to_string(),
            (1 + 3 * length).to_string(),
            "1".into(),
            "7.5".into()
        ]
    );
}
