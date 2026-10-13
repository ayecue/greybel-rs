use std::path::{Path, PathBuf};

use super::LanguageRegistry;

/// Files below a root, skipping `.git`, what `.gitignore` skips, and every language's build folders.
pub struct FileTree<'a> {
    root: &'a Path,
}

impl<'a> FileTree<'a> {
    pub fn new(root: &'a Path) -> Self {
        FileTree { root }
    }

    /// Files below `folder` (relative to the root) that `matches` accepts, relative to the root.
    pub fn files(&self, folder: &Path, matches: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
        let walker = ignore::WalkBuilder::new(self.root.join(folder))
            .hidden(false)
            .filter_entry(|entry| {
                entry.file_name() != ".git"
                    && !Self::is_build_folder(&entry.file_name().to_string_lossy())
            })
            .build();
        let mut files = walker
            .flatten()
            .filter(|entry| {
                entry.file_type().is_some_and(|kind| kind.is_file()) && matches(entry.path())
            })
            .filter_map(|entry| Some(entry.path().strip_prefix(self.root).ok()?.to_path_buf()))
            .collect::<Vec<_>>();

        files.sort();

        files
    }

    /// Checks whether `path` belongs to a language build-output directory such as `target` or `obj`.
    pub fn is_build_output(path: &Path) -> bool {
        path.components()
            .any(|component| Self::is_build_folder(&component.as_os_str().to_string_lossy()))
    }

    fn is_build_folder(name: &str) -> bool {
        LanguageRegistry::ALL
            .iter()
            .any(|language| language.build_folders().contains(&name))
    }
}
