//! Converts editor URIs into Grey Hack paths.

use greybel_documents::Uri;

use crate::protocol::SourceLocation;

/// Document-URI to Grey Hack path conversion.
pub(super) trait GameFile {
    /// Returns the filesystem path using forward slashes.
    fn game_path(&self) -> String;
}

impl GameFile for Uri {
    fn game_path(&self) -> String {
        self.fs_path().replace('\\', "/")
    }
}

impl SourceLocation {
    /// Creates a game-path source location at the given line and character 1.
    pub(super) fn in_game(path: &str, line: i32) -> Self {
        Self {
            uri: Uri::file(path).to_string(),
            line: u32::try_from(line).unwrap_or(0),
            character: 1,
        }
    }
}
