//! Snapshot tests for `tests/fixtures/greybel/`. Update with `INSTA_UPDATE=always cargo test`.

use greybel_documents_fs_provider::FsProvider;
use greybel_documents_memory_provider::MemoryProvider;
use greybel_test_support::future::block_on;
use greybel_transpiler::{
    BeautifyOptions, BuildMode, TranspileOptions, UglifyOptions, transpile, transpile_project,
    transpile_project_async,
};

use crate::support::greybel::{fixtures, modes, options, project};
use crate::support::unobfuscated;

#[test]
fn should_transpile_every_script_in_every_mode() {
    insta::glob!("fixtures/greybel/scripts/*.src", |path| {
        let source = std::fs::read_to_string(path).expect("script");
        let modes = modes();

        let out = fixtures().sections(modes, |options| transpile(&source, &options));

        insta::assert_snapshot!(out);
    });
}

#[test]
fn should_bundle_every_script_in_every_mode() {
    insta::glob!("fixtures/greybel/{scripts,special}/*.src", |path| {
        let modes = modes().map(|(title, options)| (title, unobfuscated(options)));

        let out = fixtures().sections(modes, |options| project(path, &options));

        insta::assert_snapshot!(out);
    });
}

/// Tests that strict mode rejects syntax sugar unsupported by the game.
#[test]
fn should_reject_unsupported_sugar_in_strict_mode() {
    insta::glob!("fixtures/greybel/scripts/strict-mode.src", |path| {
        let source = std::fs::read_to_string(path).expect("script");
        let cases = modes().into_iter().take(3).flat_map(|(title, options)| {
            let options = TranspileOptions {
                strict_mode: true,
                ..unobfuscated(options)
            };
            [
                (format!("direct, {title}"), (false, options.clone())),
                (format!("bundle, {title}"), (true, options)),
            ]
        });

        let out = fixtures().sections(cases, |(bundle, options)| {
            if bundle {
                project(path, &options)
            } else {
                transpile(&source, &options)
            }
        });

        insta::assert_snapshot!(out);
    });
}

#[test]
fn should_transpile_the_same_with_an_async_provider() {
    insta::glob!("fixtures/greybel/{scripts,special}/*.src", |path| {
        let options = unobfuscated(options(BuildMode::Uglify(UglifyOptions::default())));
        let entry = FsProvider::uri(path);
        let sync = transpile_project(&entry, &FsProvider::default(), &options);

        let asynchronous = block_on(transpile_project_async(
            &entry,
            &FsProvider::default(),
            &options,
        ));

        assert_eq!(
            sync.map_err(|error| error.to_string()),
            asynchronous.map_err(|error| error.to_string()),
            "{entry}"
        );
    });
}

/// Formatting keeps a string's line breaks; only output for the game joins them with `char(10)`.
#[test]
fn should_retain_multiline_strings_when_asked() {
    let source = "a = \"one\ntwo\"\n";
    let options = TranspileOptions {
        mode: BuildMode::Beautify(BeautifyOptions::default()),
        retain_multiline_strings: true,
        ..TranspileOptions::default()
    };

    let output = transpile(source, &options).expect("a transpiled script");

    assert_eq!(output, "a = \"one\ntwo\"");
}

/// `const enum` entries are written as values in every file of a build and before the enum, except in a file that
/// also uses the enum's name as a variable.
#[test]
fn should_write_const_enum_values_across_files() {
    let mut provider = MemoryProvider::new();

    provider.insert(
        "/project/main.src",
        "show = function\n  print Port.Ssh\nend function\n#include \"ports.src\"\nimport_code(\"lib.src\")\nimport_code(\"other.src\")\nshow",
    );
    provider.insert(
        "/project/ports.src",
        "const enum Port\n  Ssh = 22\n  Name = \"web\"\nend enum",
    );
    provider.insert("/project/lib.src", "print Port.Name");
    provider.insert(
        "/project/other.src",
        "print Port.Name\nf = function(Port)\n  return Port.Ssh\nend function",
    );

    let outputs = transpile_project(
        &MemoryProvider::uri("/project/main.src"),
        &provider,
        &unobfuscated(TranspileOptions {
            dialect: greybel_core::Dialect::GreyScript,
            ..TranspileOptions::default()
        }),
    )
    .expect("a build");
    let (main, lib, other) = (&outputs[0].code, &outputs[1].code, &outputs[2].code);

    assert!(main.contains("print(22)"), "{main}");
    assert!(lib.contains("print(\"web\")"), "{lib}");
    assert!(
        other.contains("print(Port.Name)") && other.contains("return Port.Ssh"),
        "{other}"
    );
}
