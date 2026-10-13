use greybel_bundler::{Build, BuildError, BuildOptions, Destination};
use greybel_documents_memory_provider::MemoryProvider;
use greybel_test_support::future::ready_output;
use greybel_transpiler::{BuildMode, UglifyOptions};

use crate::support::{MAIN, auto_compiled, build, build_with, options, paths, project, uri};

#[test]
fn should_lay_out_files_relative_to_the_entry_folder() {
    let options = options();
    let output = build(options);

    assert_eq!(paths(&output), ["main.src", "lib/tool.src"]);
}

#[test]
fn should_point_import_code_at_the_ingame_directory() {
    let options = options();
    let output = build(options);

    assert!(
        output.files[0]
            .code
            .contains("import_code(\"/root/lib/tool.src\")")
    );
}

#[test]
fn should_point_import_code_at_the_resources_with_auto_compile() {
    let options = auto_compiled(false);

    let output = build(options);

    let expected = format!(
        "import_code(\"{}/lib/tool.src\")",
        output.destination.resources
    );

    assert!(output.files[0].code.contains(&expected));
}

#[test]
fn should_rename_the_entry_output() {
    let options = BuildOptions {
        output_name: Some("app.src".to_owned()),
        ..options()
    };
    let output = build(options);

    assert_eq!(
        (paths(&output), output.entries.clone()),
        (vec!["app.src", "lib/tool.src"], vec!["app.src".to_owned()])
    );
}

#[test]
fn should_refuse_an_output_name_with_several_entries() {
    let options = BuildOptions {
        output_name: Some("app.src".to_owned()),
        ..options()
    };
    let result = build_with(options, &project(), &[MAIN, "/project/src/lib/tool.src"]);

    assert!(matches!(result, Err(BuildError::OutputNameWithEntries(2))));
}

#[test]
fn should_refuse_to_build_nothing() {
    let options = options();
    let result = build_with(options, &project(), &[]);

    assert!(matches!(result, Err(BuildError::NoEntries)));
}

#[test]
fn should_root_several_entries_at_their_common_folder() {
    let provider: MemoryProvider = [("/p/a/main.src", "print 1"), ("/p/b/main.src", "print 2")]
        .into_iter()
        .collect();
    let output =
        build_with(options(), &provider, &["/p/a/main.src", "/p/b/main.src"]).expect("builds");

    assert_eq!(
        (paths(&output), output.root.path().to_owned()),
        (vec!["a/main.src", "b/main.src"], "/p".to_owned())
    );
}

#[test]
fn should_root_at_a_folder_that_contains_imported_code_outside_the_entry_folder() {
    let provider: MemoryProvider = [
        ("/p/src/main.src", "import_code(\"../lib/tool.src\")"),
        ("/p/lib/tool.src", "print 1"),
    ]
    .into_iter()
    .collect();
    let output = build_with(options(), &provider, &["/p/src/main.src"]).expect("builds");

    assert_eq!(paths(&output), ["src/main.src", "lib/tool.src"]);
}

#[test]
fn should_retain_api_names_and_params_when_uglifying() {
    let provider: MemoryProvider = [("/p/main.src", "count = params.len\nprint count")]
        .into_iter()
        .collect();
    let options = BuildOptions {
        transpile: greybel_transpiler::TranspileOptions {
            mode: BuildMode::Uglify(UglifyOptions::default()),
            ..options().transpile
        },
        ..options()
    };
    let output = build_with(options, &provider, &["/p/main.src"]).expect("builds");
    let code = &output.files[0].code;

    assert!(
        code.contains("params") && code.contains("print") && !code.contains("count"),
        "{code}"
    );
}

#[test]
fn should_build_the_same_asynchronously() {
    let build = Build::new(options(), &mut fastrand::Rng::with_seed(7));
    let provider = project();
    let output = ready_output(build.run_async(&[uri(MAIN)], &provider)).expect("builds");

    assert_eq!(output, build.run(&[uri(MAIN)], &provider).expect("builds"));
}

#[test]
fn should_name_the_resources_folder_inside_the_ingame_directory() {
    let mut rng = fastrand::Rng::with_seed(1);
    let destination = Destination::new(" /root/ ", &mut rng);
    let name = destination
        .resources
        .strip_prefix("/root/")
        .expect("inside /root");

    assert_eq!(
        (
            destination.directory.as_str(),
            name.len(),
            destination.temporary_folder_name.len()
        ),
        ("/root", 5, 5)
    );
}
