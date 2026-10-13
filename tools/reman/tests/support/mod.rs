//! Repositories built in a temporary folder: files, a git history, and the manifests of each language.

pub mod check;
pub mod git;
pub mod init;
pub mod release;
pub mod repository;
pub mod tasks;

use std::path::Path;
use std::process::Command;

use reman::repository::Repository;
use tempfile::TempDir;

pub const ROOT_FILE: &str = r#"
[repository]
github = "owner/name"
tag = "{name}/v{version}"
changes = ".changes"

[repository.project]
skip = ["tests/**"]
"#;

pub struct Fixture {
    pub dir: TempDir,
}

impl Fixture {
    /// A git repository with the root `reman.toml`, committed.
    pub fn new() -> Self {
        let fixture = Fixture {
            dir: tempfile::tempdir().unwrap(),
        };

        fixture.git(&["init", "--quiet", "--initial-branch=main"]);
        fixture.file("reman.toml", ROOT_FILE);

        fixture
    }

    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    pub fn file(&self, path: &str, text: &str) -> &Self {
        let absolute = self.root().join(path);

        std::fs::create_dir_all(absolute.parent().unwrap()).unwrap();
        std::fs::write(absolute, text.trim_start()).unwrap();

        self
    }

    pub fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.root().join(path)).unwrap()
    }

    pub fn exists(&self, path: &str) -> bool {
        self.root().join(path).exists()
    }

    pub fn commit(&self) -> &Self {
        self.git(&["add", "--all"]);
        self.git(&[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "commit",
        ]);

        self
    }

    pub fn git(&self, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(self.root())
            .status()
            .unwrap();

        assert!(status.success(), "git {args:?}");
    }

    pub fn repository(&self) -> Repository {
        Repository::load(self.root()).unwrap()
    }

    /// A Cargo workspace at `path` whose members are its folders.
    pub fn cargo_workspace(&self, path: &str) -> &Self {
        self.file(
            &format!("{path}/Cargo.toml"),
            "[workspace]\nmembers = [\"*\"]\n\n[workspace.package]\nversion = \"0.1.0\"\n",
        );

        self.file(
            &format!("{path}/reman.toml"),
            "[workspace]\nlanguage = \"rust\"\n",
        )
    }

    /// A crate of the workspace at `workspace` with path dependencies on its siblings, and a `[project]` of `kind`
    /// (none for a development crate).
    pub fn crate_(
        &self,
        workspace: &str,
        name: &str,
        dependencies: &[&str],
        kind: Option<&str>,
    ) -> &Self {
        let dependencies = dependencies
            .iter()
            .map(|dependency| format!("{dependency} = {{ path = \"../{dependency}\" }}\n"))
            .collect::<String>();

        self.file(
            &format!("{workspace}/{name}/Cargo.toml"),
            &format!("[package]\nname = \"{name}\"\nversion.workspace = true\nedition = \"2024\"\n\n[dependencies]\n{dependencies}"),
        );
        self.file(&format!("{workspace}/{name}/src/lib.rs"), "");

        if let Some(kind) = kind {
            self.file(
                &format!("{workspace}/{name}/reman.toml"),
                &format!("[project]\nkind = \"{kind}\"\n"),
            )
            .file(
                &format!("{workspace}/{name}/CHANGELOG.md"),
                &format!("# {name}\n"),
            );
        }

        self
    }

    /// The first release record, with every project's current version.
    pub fn released(&self) -> &Self {
        let repository = self.repository();
        let versions = repository
            .projects()
            .map(|(_, node)| {
                format!(
                    "\"{}\" = \"{}\"\n",
                    node.id,
                    node.package.version.clone().unwrap_or_default()
                )
            })
            .collect::<String>();

        self.file(
            ".changes/release.toml",
            &format!("date = \"2026-01-01\"\n\n[versions]\n{versions}"),
        )
    }

    pub fn change(&self, name: &str, levels: &[(&str, &str)], note: &str) -> &Self {
        let header = levels
            .iter()
            .map(|(project, level)| format!("{project}: {level}\n"))
            .collect::<String>();

        self.file(
            &format!(".changes/pending/{name}.md"),
            &format!("---\n{header}---\n\n{note}\n"),
        )
    }
}
