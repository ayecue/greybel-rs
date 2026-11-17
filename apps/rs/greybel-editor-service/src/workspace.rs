use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A shared project: its entry file and every file by its relative path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub entry: String,
    pub files: BTreeMap<String, String>,
}

/// Why a workspace is rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WorkspaceError {
    #[error("A workspace needs at least one file.")]
    Empty,
    #[error("A workspace has at most {} files.", Workspace::MAX_FILES)]
    TooManyFiles,
    #[error(
        "The files have more than {} characters together.",
        Workspace::MAX_CHARACTERS
    )]
    TooLarge,
    #[error(
        "\"{0}\" is not a valid path: use relative paths with \"/\", without empty, \".\" or \"..\" folders."
    )]
    InvalidPath(String),
    #[error("The entry \"{0}\" is not one of the files.")]
    MissingEntry(String),
}

impl Workspace {
    /// The file of a project saved as plain code.
    pub const DEFAULT_ENTRY: &str = "main.src";
    pub const MAX_CHARACTERS: usize = 1_200_000;
    pub const MAX_FILES: usize = 500;
    pub const MAX_PATH_CHARACTERS: usize = 255;

    pub fn validate(&self) -> Result<(), WorkspaceError> {
        if self.files.is_empty() {
            return Err(WorkspaceError::Empty);
        }

        if self.files.len() > Self::MAX_FILES {
            return Err(WorkspaceError::TooManyFiles);
        }

        if let Some(path) = self.files.keys().find(|path| !Self::is_valid_path(path)) {
            return Err(WorkspaceError::InvalidPath(path.clone()));
        }

        if !self.files.contains_key(&self.entry) {
            return Err(WorkspaceError::MissingEntry(self.entry.clone()));
        }

        if self.count_characters() > Self::MAX_CHARACTERS {
            return Err(WorkspaceError::TooLarge);
        }

        Ok(())
    }

    /// The characters of all files.
    pub fn count_characters(&self) -> usize {
        self.files
            .values()
            .map(|content| content.chars().count())
            .sum()
    }

    /// SHA-256 of the entry and the files in path order, each length-prefixed.
    pub fn content_hash(&self) -> Vec<u8> {
        let mut hasher = Sha256::new();

        Self::hash_field(&mut hasher, &self.entry);

        for (path, content) in &self.files {
            Self::hash_field(&mut hasher, path);
            Self::hash_field(&mut hasher, content);
        }

        hasher.finalize().to_vec()
    }

    fn hash_field(hasher: &mut Sha256, text: &str) {
        hasher.update((text.len() as u64).to_le_bytes());
        hasher.update(text.as_bytes());
    }

    /// The same rule as the web editor's paths.
    fn is_valid_path(path: &str) -> bool {
        !path.is_empty()
            && path.chars().count() <= Self::MAX_PATH_CHARACTERS
            && !path.contains('\\')
            && path
                .split('/')
                .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
    }
}

/// The `content` text of the legacy `/code` API: plain code for a single `main.src`, otherwise the web editor's
/// project document `{"greybel": "project", "version": 1, "entry", "files"}`.
pub struct LegacyContent;

#[derive(Serialize, Deserialize)]
struct ProjectDocument {
    greybel: String,
    version: u32,
    entry: String,
    files: BTreeMap<String, String>,
}

impl LegacyContent {
    const MARKER: &str = "project";
    const VERSION: u32 = 1;

    pub fn decode(text: &str) -> Workspace {
        if text.starts_with('{')
            && let Ok(document) = serde_json::from_str::<ProjectDocument>(text)
            && document.greybel == Self::MARKER
            && document.version == Self::VERSION
            && !document.files.is_empty()
        {
            return Workspace {
                entry: document.entry,
                files: document.files,
            };
        }

        Workspace {
            entry: Workspace::DEFAULT_ENTRY.to_owned(),
            files: BTreeMap::from([(Workspace::DEFAULT_ENTRY.to_owned(), text.to_owned())]),
        }
    }

    pub fn encode(workspace: &Workspace) -> String {
        if workspace.entry == Workspace::DEFAULT_ENTRY
            && workspace.files.len() == 1
            && let Some(code) = workspace.files.get(Workspace::DEFAULT_ENTRY)
        {
            return code.clone();
        }

        serde_json::to_string(&ProjectDocument {
            greybel: Self::MARKER.to_owned(),
            version: Self::VERSION,
            entry: workspace.entry.clone(),
            files: workspace.files.clone(),
        })
        .unwrap_or_default()
    }
}
