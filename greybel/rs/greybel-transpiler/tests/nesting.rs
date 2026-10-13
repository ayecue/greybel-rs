//! Tests that parser-depth-limit code transpiles in every mode without stack overflow on native and wasm targets.

use greybel_test_support::node_depth::{assert_no_failures, programs_at_max_depth};
use greybel_transpiler::{BeautifyOptions, BuildMode, TranspileOptions, UglifyOptions, transpile};

#[test]
fn should_transpile_code_at_the_depth_limit() {
    let modes = [
        BuildMode::Default,
        BuildMode::Uglify(UglifyOptions::default()),
        BuildMode::Beautify(BeautifyOptions::default()),
    ];
    let programs = programs_at_max_depth();

    let outcomes: Vec<_> = programs
        .iter()
        .flat_map(|(shape, source)| {
            modes.iter().map(move |mode| {
                let options = TranspileOptions {
                    mode: mode.clone(),
                    ..TranspileOptions::default()
                };

                (shape, transpile(source, &options))
            })
        })
        .collect();

    assert_no_failures(outcomes);
}
