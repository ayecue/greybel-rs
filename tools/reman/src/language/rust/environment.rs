//! Folder table of the root `.cargo/config.toml` (its `[env]` entries with `relative = true`), through which
//! crates name files outside them: `"$PROTOCOLS/io"` in definition macros, `env!("TEXT_MESH_SHADERS")`.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use anyhow::{Context, Result};
use indexmap::IndexMap;
use regex::Regex;

use crate::language::{FileTree, Reference, Scope};
use crate::paths::PathExt;

#[derive(Debug, Default)]
pub struct Environment {
    /// Maps names to root-relative folders.
    pub folders: IndexMap<String, PathBuf>,
}

impl Environment {
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join(".cargo/config.toml");

        if !path.exists() {
            return Ok(Environment::default());
        }

        let text = std::fs::read_to_string(&path)?;
        let table: toml::Table =
            toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        let mut folders = IndexMap::new();

        for (name, value) in table
            .get("env")
            .and_then(toml::Value::as_table)
            .into_iter()
            .flatten()
        {
            let Some(entry) = value.as_table() else {
                continue;
            };
            let relative = entry.get("relative").and_then(toml::Value::as_bool) == Some(true);

            if let (true, Some(folder)) =
                (relative, entry.get("value").and_then(toml::Value::as_str))
            {
                folders.insert(name.clone(), Path::new(folder).normalize());
            }
        }

        Ok(Environment { folders })
    }

    /// Folder-table references in crate `member` (relative to `root`): production references from `src/` and
    /// `build.rs` what it ships, from everything else (tests, benches, examples) its tests.
    pub fn sources_of(&self, root: &Path, member: &Path) -> Result<Vec<Reference>> {
        if self.folders.is_empty() {
            return Ok(Vec::new());
        }

        let sources = FileTree::new(root).files(member, |path| {
            path.extension().is_some_and(|extension| extension == "rs")
        });
        let mut references = Vec::new();

        for file in sources {
            let inside = file.strip_prefix(member).unwrap_or(&file);
            let scope = if inside.starts_with("src") || inside == Path::new("build.rs") {
                Scope::Normal
            } else {
                Scope::Dev
            };
            let text = std::fs::read_to_string(root.join(&file))?;

            references.extend(self.references(&text, scope, &file));
        }

        Ok(references)
    }

    /// Folders a Rust source names through the table.
    fn references(&self, text: &str, scope: Scope, source: &Path) -> Vec<Reference> {
        static MACRO_PATH: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r#""\$([A-Z][A-Z0-9_]*)/([^"]*)""#).unwrap());
        static ENV: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"(?:env!|var_os|var)\(\s*"([A-Z][A-Z0-9_]*)"\s*\)(?:\s*,\s*"/?([^"]*)")?"#)
                .unwrap()
        });

        let mut references = Vec::new();
        let mut push = |name: &str, tail: &str| {
            if let Some(folder) = self.folders.get(name) {
                references.push(Reference {
                    path: folder.join(tail).normalize(),
                    scope,
                    source: source.to_owned(),
                });
            }
        };

        for captures in MACRO_PATH.captures_iter(text) {
            push(&captures[1], &captures[2]);
        }

        for captures in ENV.captures_iter(text) {
            push(
                &captures[1],
                captures.get(2).map_or("", |tail| tail.as_str()),
            );
        }

        references
    }
}
