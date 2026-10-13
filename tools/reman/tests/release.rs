use reman::release::changelog::Changelog;
use reman::release::{PublishOptions, Publisher};

use crate::support::Fixture;
use crate::support::release::{apply, diamond, plan_of, versions};

#[test]
fn should_bump_a_dependent_once_for_two_changed_dependencies() {
    // Arrange
    let fixture = diamond();

    fixture
        .change("left", &[("left", "minor")], "Left.")
        .change("right", &[("right", "patch")], "Right.");

    // Act
    let repository = fixture.repository();
    let plan = plan_of(&repository);

    // Assert
    assert_eq!(
        versions(&repository, &plan),
        [
            "rs/left 0.1.0 -> 0.2.0",
            "rs/right 0.1.0 -> 0.1.1",
            "rs/app 0.1.0 -> 0.1.1"
        ]
    );
}

#[test]
fn should_bump_by_the_furthest_level_of_its_change_files() {
    // Arrange
    let fixture = diamond();

    fixture
        .change("one", &[("app", "patch")], "One.")
        .change("two", &[("app", "major")], "Two.");

    // Act
    let repository = fixture.repository();
    let plan = plan_of(&repository);

    // Assert
    assert_eq!(versions(&repository, &plan), ["rs/app 0.1.0 -> 1.0.0"]);
}

#[test]
fn should_not_release_a_project_named_none() {
    // Arrange
    let fixture = diamond();

    fixture.change("comments", &[("core", "none")], "");

    // Act
    let repository = fixture.repository();
    let plan = plan_of(&repository);

    // Assert
    assert!(plan.releases.is_empty());
}

#[test]
fn should_prefix_a_dependency_note_with_the_project_it_was_made_in() {
    // Arrange
    let fixture = diamond();

    fixture.change("core", &[("core", "minor")], "Faster lookups.");
    apply(&fixture.repository());

    // Act
    let changelog = fixture.read("rs/app/CHANGELOG.md");

    // Assert
    assert_eq!(
        changelog,
        "# app\n\n## 0.1.1 · 2026-02-02\n\n### Patch changes\n\n- core: Faster lookups.\n"
    );
}

#[test]
fn should_write_a_change_reaching_a_project_twice_once() {
    // Arrange
    let fixture = diamond();

    fixture.change(
        "both",
        &[("left", "patch"), ("right", "patch")],
        "Both sides.",
    );
    apply(&fixture.repository());

    // Act
    let changelog = fixture.read("rs/app/CHANGELOG.md");

    // Assert
    assert_eq!(
        changelog,
        "# app\n\n## 0.1.1 · 2026-02-02\n\n### Patch changes\n\n- left, right: Both sides.\n"
    );
}

#[test]
fn should_write_an_inherited_cargo_version_as_the_crate_s_own() {
    // Arrange
    let fixture = diamond();

    fixture.change("core", &[("core", "minor")], "Core.");
    apply(&fixture.repository());

    // Act
    let manifest = fixture.read("rs/core/Cargo.toml");

    // Assert
    assert!(manifest.contains("\nversion = \"0.2.0\"\n"), "{manifest}");
}

#[test]
fn should_write_a_new_version_into_the_lock_file() {
    // Arrange
    let fixture = diamond();

    fixture.file(
        "rs/Cargo.lock",
        "version = 4\n\n[[package]]\nname = \"core\"\nversion = \"0.1.0\"\n\n[[package]]\nname = \"left\"\nversion = \"0.1.0\"\ndependencies = [\n \"core\",\n]\n",
    );
    fixture
        .commit()
        .change("core", &[("core", "minor")], "Core.");
    apply(&fixture.repository());

    // Act
    let lock = fixture.read("rs/Cargo.lock");

    // Assert
    assert!(
        lock.contains("name = \"core\"\nversion = \"0.2.0\""),
        "{lock}"
    );
}

#[test]
fn should_move_the_change_files_into_the_folder_of_the_release() {
    // Arrange
    let fixture = diamond();

    fixture.change("core", &[("core", "patch")], "Core.");
    apply(&fixture.repository());

    // Act
    let moved = (
        fixture.exists(".changes/pending/core.md"),
        fixture.read(".changes/2026-02-02/core.md"),
    );

    // Assert
    assert_eq!(
        moved,
        (false, "---\ncore: patch\n---\n\nCore.\n".to_owned())
    );
}

#[test]
fn should_number_the_folder_of_a_second_release_on_one_day() {
    // Arrange
    let fixture = diamond();

    fixture.change("core", &[("core", "patch")], "Core.");
    apply(&fixture.repository());
    fixture.change("left", &[("left", "patch")], "Left.");
    apply(&fixture.repository());

    // Act
    let exists = fixture.exists(".changes/2026-02-02-2/left.md");

    // Assert
    assert!(exists);
}

#[test]
fn should_record_the_released_versions() {
    // Arrange
    let fixture = diamond();

    fixture.change("core", &[("core", "patch")], "Core.");
    apply(&fixture.repository());

    // Act
    let record = fixture.read(".changes/release.toml");

    // Assert
    assert!(
        record.contains("\"rs/core\" = \"0.1.1\"") && record.contains("\"rs/left\" = \"0.1.1\""),
        "{record}"
    );
}

#[test]
fn should_replace_only_the_top_level_package_json_version() {
    // Arrange
    let fixture = Fixture::new();

    fixture
        .file("ts/pnpm-workspace.yaml", "packages:\n  - '*'\n")
        .file("ts/reman.toml", "[workspace]\nlanguage = \"typescript\"\n")
        .file("ts/site/package.json", "{\n  \"name\": \"site\",\n  \"version\": \"1.0.0\",\n  \"engines\": { \"version\": \"1.0.0\" }\n}\n")
        .file("ts/site/reman.toml", "[project]\nkind = \"site\"\n")
        .released()
        .commit()
        .change("site", &[("site", "minor")], "Site.");
    apply(&fixture.repository());

    // Act
    let manifest = fixture.read("ts/site/package.json");

    // Assert
    assert_eq!(
        manifest,
        "{\n  \"name\": \"site\",\n  \"version\": \"1.1.0\",\n  \"engines\": { \"version\": \"1.0.0\" }\n}\n"
    );
}

#[test]
fn should_insert_a_section_above_the_newest_release() {
    // Arrange
    let existing = "# app\n\nIntro.\n\n## 0.1.0 · 2026-01-01\n\n- First.\n";

    // Act
    let changelog = Changelog::new(existing).with_section("## 0.2.0 · 2026-02-02\n\n- Second.\n");

    // Assert
    assert_eq!(
        changelog,
        "# app\n\nIntro.\n\n## 0.2.0 · 2026-02-02\n\n- Second.\n\n## 0.1.0 · 2026-01-01\n\n- First.\n"
    );
}

#[test]
fn should_publish_an_app_without_a_tag() {
    // Arrange
    let fixture = diamond();
    let repository = fixture.repository();
    let options = PublishOptions {
        dry_run: true,
        draft: true,
        offline: true,
    };

    // Act
    let published = Publisher::new(&repository, &options)
        .unwrap()
        .publish(&mut Vec::new())
        .unwrap();

    // Assert
    assert_eq!(
        published
            .iter()
            .map(|app| app.tag.as_str())
            .collect::<Vec<_>>(),
        ["app/v0.1.0"]
    );
}

#[test]
fn should_not_publish_an_app_whose_tag_exists() {
    // Arrange
    let fixture = diamond();

    fixture.git(&[
        "-c",
        "user.name=test",
        "-c",
        "user.email=test@example.com",
        "tag",
        "-a",
        "app/v0.1.0",
        "-m",
        "app 0.1.0",
    ]);

    let repository = fixture.repository();
    let options = PublishOptions {
        dry_run: true,
        draft: true,
        offline: true,
    };

    // Act
    let published = Publisher::new(&repository, &options)
        .unwrap()
        .publish(&mut Vec::new())
        .unwrap();

    // Assert
    assert!(published.is_empty());
}

#[test]
fn should_publish_an_app_under_its_own_tag() {
    // Arrange
    let fixture = diamond();

    fixture
        .file(
            "rs/app/reman.toml",
            "[project]\nkind = \"app\"\ntag = \"tool/v{version}\"\n",
        )
        .commit();

    let repository = fixture.repository();
    let options = PublishOptions {
        dry_run: true,
        draft: true,
        offline: true,
    };

    // Act
    let published = Publisher::new(&repository, &options)
        .unwrap()
        .publish(&mut Vec::new())
        .unwrap();

    // Assert
    assert_eq!(
        published
            .iter()
            .map(|app| app.tag.as_str())
            .collect::<Vec<_>>(),
        ["tool/v0.1.0"]
    );
}

#[test]
fn should_replace_every_version_element_of_a_dotnet_package_keeping_its_layout() {
    // Arrange
    let fixture = Fixture::new();

    fixture
        .file("dotnet/Directory.Build.props", "<Project />\n")
        .file("dotnet/reman.toml", "[workspace]\nlanguage = \"dotnet\"\n")
        .file(
            "dotnet/hook/src/Hook.csproj",
            "<Project>\n  <PropertyGroup>\n    <Version>1.2.6</Version>\n    <AssemblyVersion> 1.2.6 </AssemblyVersion>\n    <Description>1.2.6</Description>\n  </PropertyGroup>\n</Project>\n",
        )
        .file("dotnet/hook/reman.toml", "[project]\nkind = \"library\"\n")
        .released()
        .commit()
        .change("hook", &[("Hook", "patch")], "Hook.");
    apply(&fixture.repository());

    // Act
    let project = fixture.read("dotnet/hook/src/Hook.csproj");

    // Assert
    assert_eq!(
        project,
        "<Project>\n  <PropertyGroup>\n    <Version>1.2.7</Version>\n    <AssemblyVersion> 1.2.7 </AssemblyVersion>\n    <Description>1.2.6</Description>\n  </PropertyGroup>\n</Project>\n"
    );
}

#[test]
fn should_read_the_notes_of_a_release_up_to_the_next() {
    // Arrange
    let text = "# app\n\n## 0.2.0 · 2026-02-02\n\n- Second.\n\n```text\n## 9.9.9 is not a heading\n```\n\n## 0.1.0 · 2026-01-01\n\n- First.\n";

    // Act
    let notes = Changelog::new(text).notes_of("0.2.0");

    // Assert
    assert_eq!(
        notes.as_deref(),
        Some("- Second.\n\n```text\n## 9.9.9 is not a heading\n```")
    );
}
