//! Tests builds, transformations, and unavailable-game failures through the builder protocol using an editor client.

use std::fmt::Write;

use duplex_core::Error;

use crate::support::editor::{Build, BuildFailure, Transform, TranspileMode};
use crate::support::requests::{build, settings};
use crate::support::{Harness, LIBRARY, MAIN};

#[test]
fn should_transform_code_as_a_build_would() {
    let mut harness = Harness::new(&[]);
    let code = "a=1\nif a==1 then print(\"one\")";
    let beautify = TranspileMode::Beautify {
        retain_parentheses: false,
        indentation_spaces: Some(2),
    };

    let beautified = harness.request(&Transform {
        code: code.into(),
        transpile: settings(beautify),
    });
    let failed = harness.request(&Transform {
        code: "x = 1 +".into(),
        transpile: settings(TranspileMode::Default),
    });

    assert_eq!(
        beautified,
        Ok("a = 1\nif a == 1 then print(\"one\")".to_owned())
    );
    assert!(
        matches!(failed, Err(Error::Failure(BuildFailure::Invalid { .. }))),
        "{failed:?}"
    );
}

#[test]
fn should_transform_code_larger_than_the_default_message_limit() {
    let mut harness = Harness::new(&[]);
    let code = (0..10_000).fold(String::new(), |mut code, index| {
        writeln!(code, "x{index}=1").expect("writing to a String cannot fail");

        code
    });

    assert!(code.len() > 64 << 10);

    let transformed = harness.request(&Transform {
        code: code.as_str().into(),
        transpile: settings(TranspileMode::Default),
    });

    assert_eq!(transformed, Ok(code.trim_end().to_owned()));
}

#[test]
fn should_build_a_project_with_the_files_it_imports_and_its_installer() {
    let main = "import_code(\"/lib.src\")\nprint(double(4))";
    let library = "double = function(n)\n  return n * 2\nend function";
    let mut harness = Harness::new(&[(MAIN, main), (LIBRARY, library)]);

    let result = harness
        .request(&build(&[MAIN], Some(160_000), None))
        .expect("built");

    assert_eq!(result.root, "file:///project");

    let paths: Vec<&str> = result.files.iter().map(|file| file.path.as_str()).collect();

    assert_eq!(paths, ["main.src", "lib.src"]);
    assert!(
        result.files[0]
            .code
            .contains("import_code(\"/root/project/lib.src\")"),
        "{}",
        result.files[0].code
    );
    assert_eq!(result.installer.len(), 1);
}

#[test]
fn should_count_an_absolute_import_from_the_root_or_the_entry_folder() {
    let nested = "file:///project/src/main.src";
    let files = [
        (nested, "import_code(\"/lib.src\")"),
        ("file:///project/src/lib.src", "x = 1"),
        (LIBRARY, "x = 2"),
    ];
    let mut harness = Harness::new(&files);
    let paths = |result: Result<_, Error<BuildFailure>>| -> Vec<String> {
        let result: crate::support::editor::BuildResult = result.expect("built");

        result.files.into_iter().map(|file| file.path).collect()
    };

    let from_entry = paths(harness.request(&build(&[nested], None, None)));
    let from_root = paths(harness.request(&Build {
        root: Some("file:///project".into()),
        ..build(&[nested], None, None)
    }));

    assert_eq!(from_entry, ["main.src", "lib.src"]);
    assert_eq!(from_root, ["src/main.src", "lib.src"]);
}

#[test]
fn should_resolve_an_import_with_the_extensions_of_the_build() {
    let library = "file:///project/lib.gs";
    let files = [(MAIN, "import_code(\"/lib\")"), (library, "x = 1")];
    let mut harness = Harness::new(&files);

    let resolved = harness.request(&build(&[MAIN], None, None));
    let unresolved = harness.request(&Build {
        extensions: vec!["src".to_owned()].into(),
        ..build(&[MAIN], None, None)
    });

    let paths: Vec<String> = resolved
        .expect("built")
        .files
        .into_iter()
        .map(|file| file.path)
        .collect();

    assert_eq!(paths, ["main.src", "lib.gs"]);
    assert!(
        matches!(
            unresolved,
            Err(Error::Failure(BuildFailure::Invalid { .. }))
        ),
        "{unresolved:?}"
    );
}

#[test]
fn should_report_where_a_build_has_a_syntax_error() {
    let mut harness = Harness::new(&[
        (MAIN, "import_code(\"/lib.src\")"),
        (LIBRARY, "a = 1\nb = ("),
    ]);

    let result = harness.request(&build(&[MAIN], None, None));

    let Err(Error::Failure(BuildFailure::Invalid {
        location: Some(location),
        ..
    })) = result
    else {
        panic!("expected a syntax error with its place, got {result:?}");
    };

    assert_eq!((location.uri.as_str(), location.line), (LIBRARY, 2));
}

#[test]
fn should_report_a_build_that_fails() {
    let mut harness = Harness::new(&[(MAIN, "import_code(\"/missing.src\")")]);

    let result = harness.request(&build(&[MAIN], None, None));

    assert!(
        matches!(result, Err(Error::Failure(BuildFailure::Invalid { .. }))),
        "{result:?}"
    );
}

#[cfg(not(feature = "game"))]
#[test]
fn should_refuse_to_deploy_into_the_game_here() {
    let mut harness = Harness::new(&[(MAIN, "print(1)")]);

    let result = harness.request(&build(&[MAIN], None, Some(7777)));

    assert_eq!(result, Err(Error::Failure(BuildFailure::Unavailable)));
}
