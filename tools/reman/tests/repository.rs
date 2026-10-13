use reman::language::Scope;
use reman::repository::Repository;

use crate::support::Fixture;
use crate::support::repository::edges;

#[test]
fn should_inherit_project_defaults_from_the_workspace_file() {
    // Arrange
    let fixture = Fixture::new();

    fixture
        .cargo_workspace("rs")
        .file(
            "rs/reman.toml",
            "[workspace]\nlanguage = \"rust\"\n\n[workspace.project]\nchangelog = \"CHANGES.md\"\n",
        )
        .crate_("rs", "core", &[], Some("library"));

    // Act
    let repository = fixture.repository();

    // Assert
    assert_eq!(
        repository.nodes[repository.resolve("core").unwrap()].changelog(),
        std::path::Path::new("rs/core/CHANGES.md")
    );
}

#[test]
fn should_leave_a_package_without_a_file_out_of_the_projects() {
    // Arrange
    let fixture = Fixture::new();

    fixture
        .cargo_workspace("rs")
        .crate_("rs", "core", &[], Some("library"))
        .crate_("rs", "test-support", &[], None);

    // Act
    let repository = fixture.repository();

    // Assert
    assert_eq!(
        repository
            .projects()
            .map(|(_, node)| node.id.as_str())
            .collect::<Vec<_>>(),
        ["rs/core"]
    );
}

#[test]
fn should_link_a_cargo_path_dependency() {
    // Arrange
    let fixture = Fixture::new();

    fixture
        .cargo_workspace("rs")
        .crate_("rs", "core", &[], Some("library"))
        .crate_("rs", "app", &["core"], Some("app"));

    // Act
    let repository = fixture.repository();

    // Assert
    assert_eq!(edges(&repository, Scope::Normal), ["rs/app -> rs/core"]);
}

#[test]
fn should_link_a_dev_dependency_for_tests_only() {
    // Arrange
    let fixture = Fixture::new();

    fixture
        .cargo_workspace("rs")
        .crate_("rs", "test-support", &[], None)
        .file(
            "rs/core/Cargo.toml",
            "[package]\nname = \"core\"\nversion = \"0.1.0\"\n\n[dev-dependencies]\ntest-support = { path = \"../test-support\" }\n",
        )
        .file("rs/core/src/lib.rs", "");

    // Act
    let repository = fixture.repository();

    // Assert
    assert_eq!(
        edges(&repository, Scope::Dev),
        ["rs/core -> rs/test-support"]
    );
}

#[test]
fn should_link_a_definition_macro_path_to_its_definition() {
    // Arrange
    let fixture = Fixture::new();

    fixture
        .file(
            ".cargo/config.toml",
            "[env]\nPROTOCOLS = { value = \"protocols\", relative = true, force = true }\n",
        )
        .file(
            "protocols/reman.toml",
            "[workspace]\nlanguage = \"duplex\"\n",
        )
        .file("protocols/io/1.0.protocol", "use core 1.0;\n")
        .cargo_workspace("rs")
        .crate_("rs", "io", &[], Some("library"))
        .file(
            "rs/io/src/lib.rs",
            "duplex::include_types!(\"$PROTOCOLS/io\", major = 1);\n",
        );

    // Act
    let repository = fixture.repository();

    // Assert
    assert_eq!(edges(&repository, Scope::Normal), ["rs/io -> duplex/io"]);
}

#[test]
fn should_link_an_env_path_joined_by_concat_to_the_package_it_names() {
    // Arrange
    let fixture = Fixture::new();

    fixture
        .file(
            ".cargo/config.toml",
            "[env]\nTOOLS = { value = \"rs\", relative = true, force = true }\nTARGET = { value = \"target\", relative = true }\n",
        )
        .cargo_workspace("rs")
        .crate_("rs", "tool", &[], Some("app"))
        .crate_("rs", "core", &[], Some("library"))
        .file(
            "rs/core/src/lib.rs",
            "const TOOL: &str = concat!(env!(\"TOOLS\"), \"/tool/src\");\nconst OUT: &str = env!(\"TARGET\");\n",
        );

    // Act
    let repository = fixture.repository();

    // Assert
    assert_eq!(edges(&repository, Scope::Normal), ["rs/core -> rs/tool"]);
    assert!(
        repository.inputs.is_empty(),
        "build output is no input: {:?}",
        repository.inputs
    );
}

#[test]
fn should_link_a_pnpm_link_override_across_workspaces() {
    // Arrange
    let fixture = Fixture::new();

    fixture
        .file("common/pnpm-workspace.yaml", "packages:\n  - '*'\n")
        .file("common/reman.toml", "[workspace]\nlanguage = \"typescript\"\n")
        .file("common/emitter/package.json", "{ \"name\": \"emitter\", \"version\": \"0.1.0\" }\n")
        .file("apps/pnpm-workspace.yaml", "packages:\n  - '*'\noverrides:\n  emitter: link:../common/emitter\n")
        .file("apps/reman.toml", "[workspace]\nlanguage = \"typescript\"\n")
        .file("apps/site/package.json", "{ \"name\": \"site\", \"version\": \"0.1.0\", \"dependencies\": { \"emitter\": \"^0.1.0\" } }\n");

    // Act
    let repository = fixture.repository();

    // Assert
    assert_eq!(edges(&repository, Scope::Normal), ["ts/site -> ts/emitter"]);
}

#[test]
fn should_link_a_dotnet_project_through_a_folder_property() {
    // Arrange
    let fixture = Fixture::new();

    fixture
        .file(
            "dotnet/Directory.Build.props",
            "<Project><PropertyGroup><Libraries>$([System.IO.Path]::GetFullPath('$(MSBuildThisFileDirectory)'))</Libraries></PropertyGroup></Project>\n",
        )
        .file("dotnet/reman.toml", "[workspace]\nlanguage = \"dotnet\"\n")
        .file("dotnet/core/src/Core.csproj", "<Project><PropertyGroup><Version>1.0.0</Version></PropertyGroup></Project>\n")
        .file(
            "dotnet/plugin/src/Plugin.csproj",
            "<Project><ItemGroup><Compile Include=\"$(Libraries)core/src/**/*.cs\" /></ItemGroup></Project>\n",
        );

    // Act
    let repository = fixture.repository();

    // Assert
    assert_eq!(
        edges(&repository, Scope::Normal),
        ["dotnet/Plugin -> dotnet/Core"]
    );
}

#[test]
fn should_reject_an_unknown_key() {
    // Arrange
    let fixture = Fixture::new();

    fixture.cargo_workspace("rs").file(
        "rs/reman.toml",
        "[workspace]\nlanguage = \"rust\"\nlanguages = 1\n",
    );

    // Act
    let loaded = Repository::load(fixture.root());

    // Assert
    assert!(loaded.is_err());
}
