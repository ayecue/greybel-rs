use greybel_documents::Uri;

use crate::destination::Destination;
use crate::options::AutoCompile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputFile {
    pub source: Uri,
    pub path: String,
    pub code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOutput {
    pub root: Uri,
    pub files: Vec<OutputFile>,
    pub entries: Vec<String>,
    pub destination: Destination,
    pub auto_compile: Option<AutoCompile>,
}

impl BuildOutput {
    /// Characters Grey Hack saves per file (`OS.MAX_CHARS_FILE`), counted in UTF-16 units like its C# strings.
    pub const GAME_FILE_LIMIT: usize = 160_000;

    /// The first file longer than `GAME_FILE_LIMIT`, with its length in UTF-16 units.
    #[must_use]
    pub fn find_oversized_file(&self) -> Option<(&OutputFile, usize)> {
        self.files.iter().find_map(|file| {
            let length = file.code.encode_utf16().count();

            (length > Self::GAME_FILE_LIMIT).then_some((file, length))
        })
    }

    #[must_use]
    pub fn file(&self, path: &str) -> Option<&OutputFile> {
        self.files.iter().find(|file| file.path == path)
    }

    pub(crate) fn upload_directory(&self) -> &str {
        if self.auto_compile.is_some() {
            &self.destination.resources
        } else {
            &self.destination.directory
        }
    }

    pub(crate) fn ingame_path(&self, file: &OutputFile) -> String {
        format!("{}/{}", self.upload_directory(), file.path)
    }
}
