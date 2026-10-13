//! Tests `greybel build` output directories and installer generation.

use crate::support::Project;
use crate::support::build::{MAIN, UTIL};

#[test]
fn should_build_a_file_with_what_it_imports_into_a_build_folder() {
    let project = Project::new(&[("main.src", MAIN), ("lib/util.src", UTIL)]);

    let run = project.run(
        &[
            "build",
            "main.src",
            "out",
            "-vr",
            "NAME=Grey",
            "--no-installer",
            "--no-auto-compile",
        ],
        "",
    );

    assert!(run.success(), "{run:?}");
    assert!(run.stdout().contains("Build done."), "{run:?}");
    assert_eq!(
        project.read("out/build/main.src"),
        "import_code(\"/root/lib/util.src\")\nprint(greet(\"Ada\"))\nprint(\"Grey\")"
    );
    assert!(
        project
            .read("out/build/lib/util.src")
            .contains("greet=function(name)")
    );
}

#[test]
fn should_uglify_and_write_installers_into_the_output_folder_itself() {
    let project = Project::new(&[("main.src", MAIN), ("lib/util.src", UTIL)]);

    let run = project.run(
        &[
            "build",
            "main.src",
            "out",
            "-u",
            "-i",
            "--no-auto-compile",
            "-dbf",
            "-si",
        ],
        "",
    );

    assert!(run.success(), "{run:?}");
    assert!(run.stdout().is_empty(), "{run:?}");
    assert!(!project.read("out/lib/util.src").contains("greet"));
    assert!(
        project
            .read("out/installer0.src")
            .contains("/root/lib/util.src")
    );
}

#[test]
fn should_build_the_files_a_glob_pattern_matches() {
    let project = Project::new(&[("a.src", "print(1)"), ("b.src", "print(2)"), ("c.txt", "x")]);

    let run = project.run(&["build", "*.src", "."], "");

    assert!(run.success(), "{run:?}");
    assert_eq!(project.read("build/a.src"), "print(1)");
    assert_eq!(project.read("build/b.src"), "print(2)");
}

#[test]
fn should_report_a_syntax_error_where_it_is() {
    let project = Project::new(&[("main.src", "print(1)\nif x then\n")]);

    let run = project.run(&["build", "main.src"], "");

    assert!(!run.success(), "{run:?}");
    assert!(run.stderr().starts_with("Build error: "), "{run:?}");
    assert!(run.stderr().contains("main.src:2:"), "{run:?}");
}

#[test]
fn should_write_installers_that_compile_the_build_by_default() {
    let project = Project::new(&[("main.src", MAIN), ("lib/util.src", UTIL)]);

    let run = project.run(&["build", "main.src"], "");

    assert!(run.success(), "{run:?}");
    assert!(
        project.read("build/installer0.src").contains("build("),
        "{run:?}"
    );
}

#[test]
fn should_build_the_entries_of_greybel_json_with_its_settings() {
    let project = Project::new(&[
        ("main.src", "print(#envar NAME)"),
        ("names.env", "NAME=Grey"),
        (
            "greybel.json",
            r#"{
                "entries": ["main.src"],
                "build": {
                    "output": "dist",
                    "mode": "Uglify",
                    "autoCompile": false,
                    "installer": { "active": false },
                    "variables": { "files": ["names.env"], "values": { "OTHER": "x" } }
                }
            }"#,
        ),
    ]);

    let run = project.run(&["build"], "");

    assert!(run.success(), "{run:?}");
    assert_eq!(
        project.read("dist/main.src"),
        "globals.I=globals\nprint(\"Grey\")"
    );
    assert!(!project.path("dist/installer0.src").exists());
}

#[test]
fn should_let_options_replace_what_greybel_json_says() {
    let project = Project::new(&[
        ("main.src", "x = 1\nprint(x)"),
        (
            "greybel.json",
            r#"{ "build": { "mode": "Uglify", "installer": { "active": false } } }"#,
        ),
    ]);

    let run = project.run(&["build", "main.src", "-b", "--no-auto-compile"], "");

    assert!(run.success(), "{run:?}");
    assert_eq!(project.read("build/main.src"), "x = 1\nprint x");
}

#[test]
fn should_build_with_the_settings_of_a_profile() {
    let project = Project::new(&[
        ("main.src", "x = 1\nprint(x)"),
        (
            "greybel.json",
            r#"{
                "build": { "autoCompile": false, "installer": { "active": false } },
                "profiles": {
                    "release": { "build": { "mode": "Uglify" } },
                    "pretty": { "extends": "release", "build": { "mode": "Beautify" } }
                }
            }"#,
        ),
    ]);

    let release = project.run(&["build", "main.src", "--profile", "release"], "");

    assert!(release.success(), "{release:?}");
    assert_eq!(
        project.read("build/main.src"),
        "globals.I=globals\nJ=1\nprint(J)"
    );

    let pretty = project.run(&["build", "main.src", "--profile", "pretty"], "");

    assert!(pretty.success(), "{pretty:?}");
    assert_eq!(project.read("build/main.src"), "x = 1\nprint x");

    let missing = project.run(&["build", "main.src", "--profile", "staging"], "");

    assert!(!missing.success(), "{missing:?}");
    assert!(
        missing
            .stderr()
            .contains("the project has no profile `staging`"),
        "{missing:?}"
    );
}

#[test]
fn should_report_a_greybel_json_it_cant_read() {
    let project = Project::new(&[
        ("main.src", "print(1)"),
        ("greybel.json", r#"{ "mode": 1 }"#),
    ]);

    let run = project.run(&["build", "main.src"], "");

    assert!(!run.success(), "{run:?}");
    assert!(run.stderr().starts_with("Project error: "), "{run:?}");
    assert!(run.stderr().contains("unknown field `mode`"), "{run:?}");
}
