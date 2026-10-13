//! Compares generated sources with checked-in files and optionally rewrites them using language-specific update
//! variables.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Display;
use std::path::{Path, PathBuf};

pub use duplex_protocol::{MinorVersionTable, VersionHistory};

/// Loads definition versions from `folder` or panics on error.
#[must_use]
pub fn versions(folder: impl AsRef<Path>) -> VersionHistory {
    let folder = folder.as_ref();

    VersionHistory::load(folder)
        .unwrap_or_else(|error| panic!("{}: the versions load: {error}", folder.display()))
}

/// Returns `major` from `definition`, caching loaded history per test thread.
///
/// # Panics
///
/// Panics if version loading fails or `major` is absent.
#[must_use]
pub fn major(definition: impl AsRef<Path>, major: u16) -> MinorVersionTable<'static> {
    thread_local! {
        static LOADED: RefCell<HashMap<PathBuf, &'static VersionHistory>> = RefCell::default();
    }

    let definition = definition.as_ref();
    let versions = LOADED.with(|histories| {
        *histories
            .borrow_mut()
            .entry(definition.to_owned())
            .or_insert_with(|| Box::leak(Box::new(versions(definition))))
    });

    versions
        .major(major)
        .unwrap_or_else(|| panic!("{}: major {major} is defined", definition.display()))
}

/// Language-specific environment variable for generated-file rewrites.
#[derive(Clone, Copy)]
enum Language {
    TypeScript,
    CSharp,
}

impl Language {
    fn variable(self) -> &'static str {
        match self {
            Self::TypeScript => "UPDATE_TS",
            Self::CSharp => "UPDATE_CS",
        }
    }
}

/// Checked-in generated files for `package`, rewritten before comparison if the language update variable is set.
pub struct GeneratedFolder {
    language: Language,
    folder: PathBuf,
    package: &'static str,
}

impl GeneratedFolder {
    /// Creates a TypeScript generated-file folder controlled by `UPDATE_TS`.
    #[must_use]
    pub fn typescript(folder: impl Into<PathBuf>, package: &'static str) -> Self {
        Self {
            language: Language::TypeScript,
            folder: folder.into(),
            package,
        }
    }

    /// Creates generated-file folder `duplex/ts/<package>/<folder>` for Rust crate `package`.
    #[must_use]
    pub fn typescript_package(package: &'static str, folder: &str) -> Self {
        Self::typescript(
            Path::new(env!("DUPLEX_TS")).join(package).join(folder),
            package,
        )
    }

    /// Creates a C# generated-file folder controlled by `UPDATE_CS`.
    #[must_use]
    pub fn csharp(folder: impl Into<PathBuf>, package: &'static str) -> Self {
        Self {
            language: Language::CSharp,
            folder: folder.into(),
            package,
        }
    }

    /// Rewrites `file` if the update variable is set, then asserts exact equality with `contents`.
    pub fn assert_up_to_date(&self, file: impl AsRef<Path>, contents: impl Display) {
        let path = self.folder.join(file);
        let contents = contents.to_string();
        let variable = self.language.variable();

        if std::env::var_os(variable).is_some() {
            if let Some(folder) = path.parent() {
                std::fs::create_dir_all(folder).expect("the generated file's folder is created");
            }

            std::fs::write(&path, &contents).expect("the generated file is written");
        }

        let stored = std::fs::read_to_string(&path).unwrap_or_default();

        assert!(
            stored == contents,
            "{} is out of date; run `{variable}=1 cargo test -p {}`",
            path.display(),
            self.package
        );
    }
}
