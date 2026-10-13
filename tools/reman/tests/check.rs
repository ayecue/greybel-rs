use reman::check::Check;

use crate::support::Fixture;
use crate::support::check::lock_with_serde;
use crate::support::release::diamond;

#[test]
fn should_report_a_changed_project_no_change_file_names() {
    // Arrange
    let fixture = diamond();

    fixture.file("rs/core/src/lib.rs", "pub fn changed() {}\n");

    // Act
    let report = Check::run(&fixture.repository(), None).unwrap();

    // Assert
    assert_eq!(report.errors.len(), 1, "{:?}", report.errors);
}

#[test]
fn should_accept_a_changed_project_named_none() {
    // Arrange
    let fixture = diamond();

    fixture
        .file("rs/core/src/lib.rs", "// A comment.\n")
        .change("comment", &[("core", "none")], "");

    // Act
    let report = Check::run(&fixture.repository(), None).unwrap();

    // Assert
    assert!(report.errors.is_empty(), "{:?}", report.errors);
}

#[test]
fn should_accept_a_change_matching_skip() {
    // Arrange
    let fixture = diamond();

    fixture.file("rs/core/tests/main.rs", "#[test]\nfn should_pass() {}\n");

    // Act
    let report = Check::run(&fixture.repository(), None).unwrap();

    // Assert
    assert!(report.errors.is_empty(), "{:?}", report.errors);
}

#[test]
fn should_report_a_project_whose_locked_dependency_changed() {
    // Arrange
    let fixture = Fixture::new();

    fixture
        .cargo_workspace("rs")
        .crate_("rs", "core", &[], Some("library"))
        .crate_("rs", "left", &[], Some("library"))
        .file("rs/Cargo.lock", &lock_with_serde("1.0.0"))
        .released()
        .commit()
        .file("rs/Cargo.lock", &lock_with_serde("1.0.1"));

    // Act
    let report = Check::run(&fixture.repository(), None).unwrap();

    // Assert
    assert!(
        report.errors.len() == 1 && report.errors[0].starts_with("rs/core changed"),
        "{:?}",
        report.errors
    );
}

#[test]
fn should_report_a_shipped_package_without_a_project() {
    // Arrange
    let fixture = Fixture::new();

    fixture
        .cargo_workspace("rs")
        .crate_("rs", "helpers", &[], None)
        .crate_("rs", "app", &["helpers"], Some("app"))
        .released()
        .commit();

    // Act
    let report = Check::run(&fixture.repository(), None).unwrap();

    // Assert
    assert_eq!(
        report.errors,
        ["rs/app ships rs/helpers (its manifest), which has no [project]"]
    );
}

#[test]
fn should_report_a_definition_named_patch_after_a_new_minor() {
    // Arrange
    let fixture = Fixture::new();

    fixture
        .file(
            "protocols/reman.toml",
            "[workspace]\nlanguage = \"duplex\"\n",
        )
        .file("protocols/io/1.0.protocol", "use core 1.0;\n")
        .file("protocols/io/reman.toml", "[project]\nkind = \"library\"\n")
        .file("protocols/io/CHANGELOG.md", "# io\n")
        .released()
        .commit()
        .file("protocols/io/1.1.protocol", "use core 1.0;\n")
        .change("io", &[("io", "patch")], "Keys.");

    // Act
    let report = Check::run(&fixture.repository(), None).unwrap();

    // Assert
    assert_eq!(
        report.errors,
        ["duplex/io moved 1.0 -> 1.1, a minor change, but is named `patch`"]
    );
}

#[test]
fn should_report_released_apps_sharing_a_tag() {
    // Arrange
    let fixture = diamond();

    fixture
        .file(
            "rs/left/reman.toml",
            "[project]\nkind = \"app\"\ntag = \"shared/v{version}\"\n",
        )
        .file(
            "rs/app/reman.toml",
            "[project]\nkind = \"app\"\ntag = \"shared/v{version}\"\n",
        )
        .commit();

    // Act
    let report = Check::run(&fixture.repository(), None).unwrap();

    // Assert
    assert_eq!(
        report.errors,
        ["released apps rs/app and rs/left share the tag shared/v{version}"]
    );
}

#[test]
fn should_accept_a_change_to_build_output() {
    // Arrange
    let fixture = diamond();

    fixture.file("rs/core/target/debug/output", "built\n");

    // Act
    let report = Check::run(&fixture.repository(), None).unwrap();

    // Assert
    assert!(report.errors.is_empty(), "{:?}", report.errors);
}

#[test]
fn should_report_a_project_without_its_changelog() {
    // Arrange
    let fixture = diamond();

    std::fs::remove_file(fixture.root().join("rs/left/CHANGELOG.md")).unwrap();

    // Act
    let report = Check::run(&fixture.repository(), None).unwrap();

    // Assert
    assert_eq!(
        report.errors,
        ["rs/left has no changelog rs/left/CHANGELOG.md (`reman init` writes one)"]
    );
}

#[test]
fn should_report_a_project_in_a_development_workspace() {
    // Arrange
    let fixture = Fixture::new();

    fixture
        .file("fixtures/echo/1.0.protocol", "use core 1.0;\n")
        .file("fixtures/ping/1.0.protocol", "use core 1.0;\n")
        .file(
            "fixtures/reman.toml",
            "[workspace]\nlanguage = \"duplex\"\ndevelopment = true\n",
        )
        .file(
            "fixtures/echo/reman.toml",
            "[project]\nkind = \"library\"\n",
        )
        .file("fixtures/echo/CHANGELOG.md", "# echo\n")
        .released()
        .commit();

    // Act
    let report = Check::run(&fixture.repository(), None).unwrap();

    // Assert
    assert_eq!(
        report.errors,
        ["duplex/echo is a [project] in fixtures, a workspace for development only"]
    );
}
