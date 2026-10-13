use std::path::Path;

use reman::config::Kind;
use reman::init::Initializer;

use crate::support::Fixture;
use crate::support::init::{options, workspace};

#[test]
fn should_guess_an_app_for_a_package_building_a_program() {
    // Arrange
    let fixture = workspace();

    // Act
    let outcome = Initializer::run(fixture.root(), &options(&[], &[])).unwrap();

    // Assert
    assert_eq!(
        outcome
            .edits
            .read(Path::new("rs/cli/reman.toml"))
            .unwrap()
            .unwrap(),
        "[project]\nkind = \"app\"\n"
    );
}

#[test]
fn should_guess_a_library_for_a_package_building_none() {
    // Arrange
    let fixture = workspace();

    // Act
    let outcome = Initializer::run(fixture.root(), &options(&[], &[])).unwrap();

    // Assert
    assert_eq!(
        outcome
            .edits
            .read(Path::new("rs/core/reman.toml"))
            .unwrap()
            .unwrap(),
        "[project]\nkind = \"library\"\n"
    );
}

#[test]
fn should_leave_an_unpublished_package_without_a_project() {
    // Arrange
    let fixture = workspace();

    // Act
    let outcome = Initializer::run(fixture.root(), &options(&[], &[])).unwrap();

    // Assert
    assert_eq!(
        outcome
            .edits
            .read(Path::new("rs/helpers/reman.toml"))
            .unwrap(),
        None
    );
}

#[test]
fn should_make_a_package_a_site_its_glob_matches() {
    // Arrange
    let fixture = workspace();

    // Act
    let outcome =
        Initializer::run(fixture.root(), &options(&[], &[(Kind::Site, "rs/c*")])).unwrap();

    // Assert
    assert_eq!(
        outcome
            .edits
            .read(Path::new("rs/core/reman.toml"))
            .unwrap()
            .unwrap(),
        "[project]\nkind = \"site\"\n"
    );
}

#[test]
fn should_write_a_workspace_file_for_a_cargo_workspace() {
    // Arrange
    let fixture = workspace();

    // Act
    let outcome = Initializer::run(fixture.root(), &options(&[], &[])).unwrap();

    // Assert
    assert_eq!(
        outcome
            .edits
            .read(Path::new("rs/reman.toml"))
            .unwrap()
            .unwrap(),
        "[workspace]\nlanguage = \"rust\"\n"
    );
}

#[test]
fn should_leave_out_a_package_named_dev() {
    // Arrange
    let fixture = workspace();

    // Act
    let outcome = Initializer::run(fixture.root(), &options(&["core"], &[])).unwrap();

    // Assert
    assert_eq!(
        outcome.edits.read(Path::new("rs/core/reman.toml")).unwrap(),
        None
    );
}

#[test]
fn should_run_on_a_repository_without_files() {
    // Arrange
    let fixture = Fixture::new();

    std::fs::remove_file(fixture.root().join("reman.toml")).unwrap();

    // Act
    let outcome = Initializer::run(fixture.root(), &options(&[], &[]));

    // Assert
    assert!(outcome.is_ok());
}

#[test]
fn should_make_a_project_of_a_package_whatever_its_folder_is_named() {
    // Arrange
    let fixture = workspace();

    // Act
    let outcome = Initializer::run(fixture.root(), &options(&[], &[])).unwrap();

    // Assert
    assert_eq!(
        outcome
            .edits
            .read(Path::new("rs/fixtures/reman.toml"))
            .unwrap()
            .unwrap(),
        "[project]\nkind = \"library\"\n"
    );
}

#[test]
fn should_leave_out_a_package_a_dev_glob_matches() {
    // Arrange
    let fixture = workspace();

    // Act
    let outcome = Initializer::run(fixture.root(), &options(&["**/fixtures"], &[])).unwrap();

    // Assert
    assert_eq!(
        outcome
            .edits
            .read(Path::new("rs/fixtures/reman.toml"))
            .unwrap(),
        None
    );
}

#[test]
fn should_give_the_kind_of_the_first_matching_pattern() {
    // Arrange
    let fixture = workspace();

    // Act
    let outcome = Initializer::run(
        fixture.root(),
        &options(&[], &[(Kind::Library, "cli"), (Kind::App, "rs/*")]),
    )
    .unwrap();

    // Assert
    assert_eq!(
        outcome
            .edits
            .read(Path::new("rs/cli/reman.toml"))
            .unwrap()
            .unwrap(),
        "[project]\nkind = \"library\"\n"
    );
}

#[test]
fn should_write_the_repository_file_with_its_defaults() {
    // Arrange
    let fixture = Fixture::new();

    std::fs::remove_file(fixture.root().join("reman.toml")).unwrap();

    // Act
    let outcome = Initializer::run(fixture.root(), &options(&[], &[])).unwrap();

    // Assert
    assert_eq!(
        outcome
            .edits
            .read(Path::new("reman.toml"))
            .unwrap()
            .unwrap(),
        "[repository]\ngithub = \"owner/name\"\ntag = \"{name}/v{version}\"\nchanges = \".changes\"\n"
    );
}

#[test]
fn should_write_an_empty_changelog_for_each_project_without_one() {
    // Arrange
    let fixture = workspace();

    // Act
    let outcome = Initializer::run(fixture.root(), &options(&[], &[])).unwrap();

    // Assert
    assert_eq!(
        outcome
            .edits
            .read(Path::new("rs/core/CHANGELOG.md"))
            .unwrap()
            .unwrap(),
        "# core\n"
    );
}

#[test]
fn should_leave_an_existing_changelog_as_it_is() {
    // Arrange
    let fixture = workspace();

    fixture.file("rs/core/CHANGELOG.md", "# core\n\n## 0.1.0\n");

    // Act
    let outcome = Initializer::run(fixture.root(), &options(&[], &[])).unwrap();

    // Assert
    assert!(
        !outcome
            .edits
            .paths()
            .any(|path| path == Path::new("rs/core/CHANGELOG.md"))
    );
}

#[test]
fn should_leave_out_a_dotnet_package_whose_projects_are_not_packable() {
    // Arrange
    let fixture = Fixture::new();

    fixture
        .file(
            "dotnet/tool/src/Tool.csproj",
            "<Project><PropertyGroup><IsPackable>false</IsPackable></PropertyGroup></Project>\n",
        )
        .file(
            "dotnet/tool/tests/Tool.Tests.csproj",
            "<Project><PropertyGroup><IsTestProject>true</IsTestProject></PropertyGroup></Project>\n",
        )
        .file("dotnet/tool/Tool.slnx", "<Solution />\n")
        .file("dotnet/reman.toml", "[workspace]\nlanguage = \"dotnet\"\n")
        .commit();

    // Act
    let outcome = Initializer::run(fixture.root(), &options(&[], &[])).unwrap();

    // Assert
    assert!(outcome.report.contains("dotnet/Tool"), "{}", outcome.report);
}

#[test]
fn should_leave_out_the_packages_of_a_development_workspace() {
    // Arrange
    let fixture = Fixture::new();

    fixture
        .file("fixtures/echo/1.0.protocol", "use core 1.0;\n")
        .file("fixtures/ping/1.0.protocol", "use core 1.0;\n")
        .file(
            "fixtures/reman.toml",
            "[workspace]\nlanguage = \"duplex\"\ndevelopment = true\n",
        )
        .commit();

    // Act
    let outcome = Initializer::run(fixture.root(), &options(&[], &[])).unwrap();

    // Assert
    assert_eq!(
        outcome
            .edits
            .paths()
            .filter(|path| path.starts_with("fixtures"))
            .count(),
        0
    );
}
