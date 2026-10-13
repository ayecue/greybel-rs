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
