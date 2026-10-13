//! Tests that code at the parser's depth limit compiles without stack overflow on native and wasm targets.

use greybel_core::{Dialect, ParserOptions};
use greybel_documents::{DocumentOptions, Uri, Workspace};
use greybel_interpreter::{CompileOptions, Program};
use greybel_test_support::node_depth::{assert_no_failures, programs_at_max_depth};

#[test]
fn should_compile_code_at_the_depth_limit() {
    let programs = programs_at_max_depth();

    let outcomes: Vec<_> = programs
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

            (shape, compiled)
        })
        .collect();

    assert_no_failures(outcomes);
}
