//! Compares repository-root-relative paths component by component and applies globs.

use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};

pub trait PathExt {
    /// Path with `.` and `..` resolved without touching the file system.
    fn normalize(&self) -> PathBuf;

    /// Uses `/` separators for glob matching and output.
    fn to_slash(&self) -> String;
}

impl PathExt for Path {
    fn normalize(&self) -> PathBuf {
        let mut normal = PathBuf::new();

        for component in self.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    if !normal.pop() {
                        normal.push("..");
                    }
                }
                other => normal.push(other),
            }
        }

        normal
    }

    fn to_slash(&self) -> String {
        let parts = self
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>();

        parts.join("/")
    }
}

/// Glob matcher for `/`-separated paths.
pub struct GlobMatcher {
    set: GlobSet,
}

impl GlobMatcher {
    pub fn new(patterns: impl IntoIterator<Item = impl AsRef<str>>) -> Result<Self> {
        let mut builder = GlobSetBuilder::new();

        for pattern in patterns {
            let pattern = pattern.as_ref();

            builder.add(Glob::new(pattern).with_context(|| format!("the glob `{pattern}`"))?);
        }

        Ok(GlobMatcher {
            set: builder.build()?,
        })
    }

    pub fn matches(&self, path: impl AsRef<Path>) -> bool {
        self.set.is_match(path)
    }
}
