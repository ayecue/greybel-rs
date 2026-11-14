//! Grey Hack-compatible file operations: symlink-aware paths, replacement, tree copies and moves,
//! sizes, and permissions. All operations access node rows.

use std::rc::Rc;

use greyhack_behaviour::LibraryKind;

use crate::storage::nodes::NodeSpec;
use crate::storage::*;
use crate::systems::*;
use crate::*;

pub mod changes;
pub mod copy;
pub mod delete;
pub mod libraries;
pub mod lookup;
pub mod permissions;
pub mod script;
pub mod sizes;
pub mod write;

/// File-system services.
pub struct FileSystems {
    pub(crate) node_rows: Rc<NodeRepository>,
    pub(crate) library_rows: Rc<LibraryIdentityRepository>,
    pub changes: Dep<FileChanges>,
    pub lookup: Dep<FileLookup>,
    pub permissions: Dep<FilePermissions>,
    pub sizes: Dep<FileSizes>,
    pub libraries: Dep<LibraryFiles>,
    pub scripts: Dep<ScriptFiles>,
}

impl FileSystems {
    pub(crate) fn new(
        registry: &Rc<greyhack_world_modules::Registry>,
        node_rows: Rc<NodeRepository>,
        library_rows: Rc<LibraryIdentityRepository>,
    ) -> Self {
        Self {
            node_rows,
            library_rows,
            changes: registry.get(),
            lookup: registry.get(),
            permissions: registry.get(),
            sizes: registry.get(),
            libraries: registry.get(),
            scripts: registry.get(),
        }
    }
}

impl AccountFiles for FileSystems {
    fn root(&self, computer: ComputerId) -> crate::HookFuture<'_, Option<crate::Node>> {
        Box::pin(async move { self.lookup.root(computer).await })
    }

    fn file<'a>(
        &'a self,
        computer: ComputerId,
        path: &'a str,
        follow: bool,
    ) -> crate::HookFuture<'a, Option<crate::Node>> {
        Box::pin(async move { self.lookup.file(computer, path, follow).await })
    }

    fn folder<'a>(
        &'a self,
        computer: ComputerId,
        path: &'a str,
        follow: bool,
    ) -> crate::HookFuture<'a, Option<crate::Node>> {
        Box::pin(async move { self.lookup.folder(computer, path, follow).await })
    }

    fn folders<'a>(&'a self, folder: &'a crate::Node) -> crate::HookFuture<'a, Vec<crate::Node>> {
        Box::pin(async move { self.lookup.folders(folder).await })
    }

    fn add_node<'a>(
        &'a self,
        folder: &'a crate::Node,
        node: &'a crate::NodeDraft,
    ) -> crate::HookFuture<'a, crate::Node> {
        Box::pin(async move { self.changes.add_node(folder, node).await })
    }

    fn remove_node<'a>(&'a self, node: &'a crate::Node) -> crate::HookFuture<'a, bool> {
        Box::pin(async move { self.changes.remove_node(node).await })
    }

    fn save_node<'a>(&'a self, node: &'a crate::Node) -> crate::HookFuture<'a, ()> {
        Box::pin(async move { self.changes.save_node(node).await })
    }
}

impl FileSystems {
    /// Returns the node at `path` on `computer`, if one exists.
    pub async fn node_at(&self, computer: ComputerId, path: &str) -> Result<Option<Node>> {
        self.node_rows.node_at(computer, path).await
    }

    /// Returns the direct children of `folder` on `computer`.
    pub async fn children(&self, computer: ComputerId, folder: NodeId) -> Result<Vec<Node>> {
        self.node_rows.children(computer, folder).await
    }

    /// Inserts a node with `fields` below `parent` and returns its ID.
    pub async fn insert_node(
        &self,
        computer: ComputerId,
        parent: Option<(NodeId, &str)>,
        name: &str,
        fields: &NodeSpec<'_>,
    ) -> Result<NodeId> {
        self.node_rows
            .insert_node(computer, parent, name, fields)
            .await
    }

    /// Removes node `id` from `computer`.
    pub async fn remove_node(&self, computer: ComputerId, id: NodeId) -> Result<()> {
        self.node_rows.remove_node(computer, id).await
    }

    /// Stores `draft` on `computer`, resolving library IDs with `library_id`.
    pub async fn insert_filesystem_with<L>(
        &self,
        computer: ComputerId,
        draft: &FileSystemDraft<L>,
        library_id: impl AsyncFn(&L) -> Result<LibraryId>,
    ) -> Result<()> {
        self.node_rows
            .insert_filesystem_with(computer, draft, library_id)
            .await
    }

    /// Returns the ID of `kind` at `version`, if it is registered.
    pub async fn library_of(&self, kind: LibraryKind, version: &str) -> Result<Option<LibraryId>> {
        self.library_rows.library_of(kind, version).await
    }

    /// Returns the newest registered library ID of `kind`, if any.
    pub async fn latest_library(&self, kind: LibraryKind) -> Result<Option<LibraryId>> {
        self.library_rows.latest_library(kind).await
    }

    /// Returns or creates the library identity of `kind` at `version`.
    pub async fn library_row(&self, kind: LibraryKind, version: &str) -> Result<LibraryId> {
        self.library_rows.library_row(kind, version).await
    }

    /// Returns the library identity linked to `node`, if any.
    pub async fn node_library(&self, node: NodeId) -> Result<Option<LibraryId>> {
        self.node_rows.node_library(node).await
    }

    /// Links `node` to `library`, or clears that link when `library` is `None`.
    pub async fn set_node_library(&self, node: NodeId, library: Option<LibraryId>) -> Result<()> {
        self.node_rows.set_node_library(node, library).await
    }
}
