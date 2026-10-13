//! Snapshot tests for `tests/fixtures/greyscript/`. Update with `INSTA_UPDATE=always cargo test`.

use greybel_documents_fs_provider::FsProvider;
use greybel_transpiler::{transpile, transpile_project};

use crate::support::greyscript::{fixtures, modes};
use crate::support::unobfuscated;

#[test]
fn should_transpile_every_script_in_every_mode() {
    insta::glob!("fixtures/greyscript/scripts/*.src", |path| {
        let source = std::fs::read_to_string(path).expect("script");
        let modes = modes();

        let out = fixtures().sections(modes, |options| transpile(&source, &options));

        insta::assert_snapshot!(out);
    });
}

/// Bundles each script with its dependencies; `import_code` files become separate outputs.
#[test]
fn should_bundle_every_script_in_every_mode() {
    insta::glob!("fixtures/greyscript/{scripts,special}/*.src", |path| {
        let entry = FsProvider::uri(path);
        let modes = modes()
            .into_iter()
            .take(3)
            .map(|(title, options)| (title, unobfuscated(options)));

        let out = fixtures().sections(modes, |options| {
            transpile_project(&entry, &FsProvider::default(), &options)
                .map(|outputs| fixtures().outputs(outputs))
        });

        insta::assert_snapshot!(out);
    });
}
