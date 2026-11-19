//! Tests that parser-depth-limit code transpiles in every mode without stack overflow on native and wasm targets.

use greybel_test_support::node_depth::{
    CHAIN_LENGTH, assert_no_failures, operator_chains, programs_at_max_depth,
};
use greybel_transpiler::{BeautifyOptions, BuildMode, TranspileOptions, Transpiler, UglifyOptions};

fn modes() -> [BuildMode; 3] {
    [
        BuildMode::Default,
        BuildMode::Uglify(UglifyOptions::default()),
        BuildMode::Beautify(BeautifyOptions::default()),
    ]
}

fn transpile(source: &str, mode: &BuildMode) -> Result<String, greybel_transpiler::TranspileError> {
    let options = TranspileOptions {
        mode: mode.clone(),
        ..TranspileOptions::default()
    };

    Transpiler::new(&options).transpile(source)
}

#[test]
fn should_transpile_operator_chains_of_any_length() {
    let programs = operator_chains(CHAIN_LENGTH);
    let modes = modes();

    let outcomes: Vec<_> = programs
        .iter()
        .flat_map(|(shape, source)| {
            modes
                .iter()
                .map(move |mode| (shape, transpile(source, mode)))
        })
        .collect();

    assert_no_failures(outcomes);
}

#[test]
fn should_write_an_operator_chain_in_order() {
    let source = "x = a + b * c - d / e and f or g";

    let outputs: Vec<String> = modes()
        .iter()
        .map(|mode| transpile(source, mode).unwrap())
        .collect();

    assert_eq!(outputs[0], "x=a+b*c-d/e and f or g");
    assert_eq!(outputs[2], source);
}

#[test]
fn should_transpile_code_at_the_depth_limit() {
    let modes = modes();
    let programs = programs_at_max_depth();

    let outcomes: Vec<_> = programs
        .iter()
        .flat_map(|(shape, source)| {
            modes.iter().map(move |mode| {
                let options = TranspileOptions {
                    mode: mode.clone(),
                    ..TranspileOptions::default()
                };

                (shape, Transpiler::new(&options).transpile(source))
            })
        })
        .collect();

    assert_no_failures(outcomes);
}
