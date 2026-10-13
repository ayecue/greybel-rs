//! Library lookup from file contents and `/lib` versions.

use crate::{ComputerId, Library, Node, Result, World};

impl World {
    /// Returns the library at `path` if the file contains library data.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn file_library(
        &self,
        computer: ComputerId,
        path: &str,
        follow: bool,
    ) -> Result<Option<Library>> {
        match self.file(computer, path, follow).await? {
            Some(node) => self.library_of(&node).await,
            None => Ok(None),
        }
    }

    /// Returns the newest library of `kind`, matching `VersionsControl.lastLibs` for hidden-zone
    /// scans.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn latest_library(
        &self,
        kind: greyhack_behaviour::LibraryKind,
    ) -> Result<Option<Library>> {
        match self.db.latest_library(kind).await? {
            Some(library) => self.db.library(library).await,
            None => Ok(None),
        }
    }

    /// Returns `node`'s library if it is a library file.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn library_of(&self, node: &Node) -> Result<Option<Library>> {
        match self.db.node_library(node.id).await? {
            Some(library) => self.db.library(library).await,
            None => Ok(None),
        }
    }

    /// Associates `node` with the world library matching `library` kind and version, or removes its
    /// library association.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn set_library_of(&self, node: &Node, library: Option<&Library>) -> Result<()> {
        let library = match library {
            Some(library) => Some(self.db.library_id(library).await?),
            None => None,
        };

        self.db.set_node_library(node.id, library).await
    }
}
