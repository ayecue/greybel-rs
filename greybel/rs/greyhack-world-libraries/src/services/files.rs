//! Full library entities loaded through core's file-to-library identity links.

use greyhack_behaviour::LibraryKind;
use greyhack_world_core::{CallFuture, LibraryVersionService, WorldRef};

use crate::{ComputerId, Library, LibraryId, Node, Result};

/// Library-domain lookups that add zones and weaknesses to core's library identities.
impl Generator {
    pub async fn full_file_library(
        &self,
        computer: ComputerId,
        path: &str,
        follow: bool,
    ) -> Result<Option<Library>> {
        match self.files.lookup.file(computer, path, follow).await? {
            Some(node) => self.full_library_of(&node).await,
            None => Ok(None),
        }
    }

    pub async fn latest_full_library(&self, kind: LibraryKind) -> Result<Option<Library>> {
        match self.files.latest_library(kind).await? {
            Some(library) => self.libraries.library(library).await,
            None => Ok(None),
        }
    }

    pub async fn full_library_of(&self, node: &Node) -> Result<Option<Library>> {
        match self.files.node_library(node.id).await? {
            Some(library) => self.libraries.library(library).await,
            None => Ok(None),
        }
    }

    pub async fn set_full_library_version(
        &self,
        computer: ComputerId,
        path: &str,
        version: &str,
    ) -> Result<bool> {
        let Some(node) = self.files.lookup.file(computer, path, false).await? else {
            return Ok(false);
        };
        let Some(mut library) = self.full_library_of(&node).await? else {
            return Ok(false);
        };

        library.version = version.to_owned();
        self.set_full_library_of(&node, Some(&library)).await?;

        Ok(true)
    }

    pub async fn add_full_node(&self, folder: &Node, node: &crate::NodeDraft) -> Result<Node> {
        let libraries = self.libraries.clone();

        self.files
            .changes
            .add_node_with(folder, node, async move |library| {
                libraries.library_id(library).await
            })
            .await
    }

    pub async fn set_full_library_of(&self, node: &Node, library: Option<&Library>) -> Result<()> {
        let library = match library {
            Some(library) => Some(self.libraries.library_id(library).await?),
            None => None,
        };

        self.files.set_node_library(node.id, library).await
    }
}

/// A world service with only its declared dependencies.
pub struct Generator {
    pub(crate) libraries: std::rc::Rc<crate::storage::LibraryRepository>,
    pub(crate) files: greyhack_world_core::Dep<greyhack_world_core::FileSystemService>,
}

impl Generator {
    pub(crate) fn new(
        libraries: std::rc::Rc<crate::storage::LibraryRepository>,
        files: greyhack_world_core::Dep<greyhack_world_core::FileSystemService>,
    ) -> Self {
        Self { libraries, files }
    }
}

impl Generator {
    /// Stores a generated library identity and its zones for world creation.
    pub async fn library_id(&self, library: &Library) -> Result<LibraryId> {
        self.libraries.library_id(library).await
    }
}

impl LibraryVersionService for Generator {
    fn set_library_version<'a>(
        &'a self,
        _: &'a WorldRef,
        computer: ComputerId,
        path: &'a str,
        version: &'a str,
    ) -> CallFuture<'a, bool> {
        Box::pin(async move { self.set_full_library_version(computer, path, version).await })
    }
}
