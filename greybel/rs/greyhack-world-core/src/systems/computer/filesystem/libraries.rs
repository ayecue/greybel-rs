//! Library lookup from file contents and `/lib` versions.

use std::rc::Rc;

use greyhack_world_modules::Dep;

use crate::storage::*;
use crate::systems::{LibraryFileService as LibraryFilesService, *};
use crate::*;

impl LibraryFilesService {
    /// Returns the kind and version of stored library `id`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn library_identity(&self, id: LibraryId) -> Result<Option<Library>> {
        self.library_rows.library_identity(id).await
    }

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
        match self.lookup.file(computer, path, follow).await? {
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
        match self.library_rows.latest_library(kind).await? {
            Some(library) => self.library_rows.library_identity(library).await,
            None => Ok(None),
        }
    }

    /// Returns `node`'s library if it is a library file.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn library_of(&self, node: &Node) -> Result<Option<Library>> {
        match self.node_rows.node_library(node.id).await? {
            Some(library) => self.library_rows.library_identity(library).await,
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
            Some(library) => Some(self.library_rows.library_identity_id(library).await?),
            None => None,
        };

        self.node_rows.set_node_library(node.id, library).await
    }
}

/// Links between library identities and files.
pub struct LibraryFileService {
    pub(crate) node_rows: Rc<NodeRepository>,
    pub(crate) library_rows: Rc<LibraryIdentityRepository>,
    pub(crate) lookup: Dep<FileLookup>,
}

impl LibraryFileService {
    pub(crate) fn new(
        registry: &Rc<greyhack_world_modules::Registry>,
        node_rows: Rc<NodeRepository>,
        library_rows: Rc<LibraryIdentityRepository>,
    ) -> Self {
        Self {
            node_rows,
            library_rows,
            lookup: registry.get(),
        }
    }
}
