use std::sync::LazyLock;

use serde::Deserialize;

/// Initial `BaseFileSystem` filesystem tree shared by new computers.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FileSystemLayout {
    #[serde(rename = "ageMinutes", default = "default_age_minutes")]
    pub age_minutes: u64,
    pub folders: Vec<FolderLayout>,
}

const fn default_age_minutes() -> u64 {
    1440
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FolderLayout {
    #[serde(rename = "nombre")]
    pub name: String,
    #[serde(rename = "ageMinutes", default = "default_age_minutes")]
    pub age_minutes: u64,
    #[serde(rename = "permisos")]
    pub permissions: String,
    pub files: Vec<FileLayout>,
    pub folders: Vec<FolderLayout>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FileLayout {
    #[serde(rename = "nombre")]
    pub name: String,
    #[serde(rename = "ageMinutes", default = "default_age_minutes")]
    pub age_minutes: u64,
    #[serde(rename = "permisos")]
    pub permissions: String,
    #[serde(rename = "proceso")]
    pub process: String,
    #[serde(rename = "isExec")]
    pub executable: bool,
    #[serde(rename = "isBinario")]
    pub binary: bool,
    #[serde(rename = "contenido")]
    pub content: String,
    /// Numeric `Fichero.TypeFile` value: 0 is generic, 1 script, 2 system log, 3 library, and so on.
    #[serde(rename = "typeFile")]
    pub file_type: u8,
}

impl FileSystemLayout {
    #[must_use]
    pub fn base() -> &'static Self {
        static BASE: LazyLock<FileSystemLayout> = LazyLock::new(|| {
            serde_json::from_str(
                crate::bundle::text("config/base_file_system.json").trim_start_matches('\u{feff}'),
            )
            .expect("the bundled file-system layout is valid")
        });

        &BASE
    }

    #[must_use]
    pub fn folder(&self, name: &str) -> Option<&FolderLayout> {
        self.folders.iter().find(|folder| folder.name == name)
    }
}

impl FolderLayout {
    #[must_use]
    pub fn folder(&self, name: &str) -> Option<&Self> {
        self.folders.iter().find(|folder| folder.name == name)
    }

    #[must_use]
    pub fn file(&self, name: &str) -> Option<&FileLayout> {
        self.files.iter().find(|file| file.name == name)
    }
}
