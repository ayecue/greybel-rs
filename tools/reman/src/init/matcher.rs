//! Packages and folders `reman init`'s `--kind` and `--dev` patterns name: ids, names, or globs of folders.

use std::path::Path;

use anyhow::Result;

use super::InitOptions;
use crate::config::Kind;
use crate::paths::GlobMatcher;
use crate::repository::Node;

/// Selects packages by ID, name, or folder glob.
pub struct Matcher {
    names: Vec<String>,
    globs: GlobMatcher,
}

impl Matcher {
    pub fn new<'a>(patterns: impl IntoIterator<Item = &'a String>) -> Result<Self> {
        let names = patterns.into_iter().cloned().collect::<Vec<_>>();

        Ok(Matcher {
            globs: GlobMatcher::new(&names)?,
            names,
        })
    }

    pub fn matches(&self, node: &Node) -> bool {
        self.names
            .iter()
            .any(|name| *name == node.id || *name == node.name)
            || self.matches_path(node.path())
    }

    pub fn matches_path(&self, path: &Path) -> bool {
        self.globs.matches(path)
    }
}

/// Package project kind from command-line options.
pub struct KindTable {
    kinds: Vec<(Kind, Matcher)>,
    dev: Matcher,
}

impl KindTable {
    pub fn new(options: &InitOptions) -> Result<Self> {
        let kinds = options
            .kinds
            .iter()
            .map(|rule| Ok((rule.kind, Matcher::new([&rule.pattern])?)))
            .collect::<Result<Vec<_>>>()?;

        Ok(KindTable {
            kinds,
            dev: Matcher::new(&options.dev)?,
        })
    }

    /// Development-only workspace folder status.
    pub fn is_dev_folder(&self, folder: &Path) -> bool {
        self.dev.matches_path(folder)
    }

    /// Kind a package's project gets, or none for a development package: `--dev` matches it, or no `--kind`
    /// pattern does and its manifest marks it unpublished; else `app` when it builds a program, `library` when not.
    pub fn kind_of(&self, node: &Node) -> Option<Kind> {
        let pattern_kind = self
            .kinds
            .iter()
            .find(|(_, matcher)| matcher.matches(node))
            .map(|(kind, _)| *kind);

        match pattern_kind {
            _ if self.dev.matches(node) => None,
            Some(kind) => Some(kind),
            None if node.package.unpublished => None,
            None if node.package.binary => Some(Kind::App),
            None => Some(Kind::Library),
        }
    }
}
