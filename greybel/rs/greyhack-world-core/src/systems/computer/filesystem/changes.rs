//! Node creation, updates, deletion, and moves.

use std::rc::Rc;

use greyhack_world_modules::Dep;

use crate::storage::nodes::NodeSpec;
use crate::storage::{ComputerRepository, LibraryIdentityRepository, NodeRepository};
use crate::systems::{
    FileChangeService as FileChangesService, FileLookup, FilePermissionService, FileSizeService,
    LibraryFileService, UserService,
};
use crate::{LibraryId, Node, NodeDraft, Result, WorldRef};

impl FileChangesService {
    /// Adds `node` to `folder`, replacing a same-named entry and storing its library. Returns the
    /// stored node.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn add_node(&self, folder: &Node, node: &NodeDraft) -> Result<Node> {
        let db = self.library_rows.clone();

        self.add_node_with(folder, node, async move |library| {
            db.library_identity_id(library).await
        })
        .await
    }

    /// Adds `node`, resolving a full library through `library_id` before linking it to the new row.
    pub async fn add_node_with<L>(
        &self,
        folder: &Node,
        node: &NodeDraft<L>,
        library_id: impl AsyncFn(&L) -> Result<LibraryId>,
    ) -> Result<Node>
    where
        L: Clone,
    {
        if let Some(same_name) = self.lookup.child(folder, &node.name).await? {
            self.node_rows
                .remove_node(folder.computer, same_name.id)
                .await?;
        }

        let mut node = node.clone();

        // Player-created files and folders use the world clock (`PlayerUtils.CrearArchivo`,
        // `CrearCarpeta`).
        node.timestamp = self.world.time_seconds();

        let id = self
            .node_rows
            .insert_node(
                folder.computer,
                Some((folder.id, &folder.path)),
                &node.name,
                &NodeSpec::of_draft(&node),
            )
            .await?;
        let new_node = self
            .node_rows
            .node(folder.computer, id)
            .await?
            .expect("the node just inserted");

        if let Some(library) = node.library() {
            let library = library_id(library).await?;

            self.node_rows
                .set_node_library(new_node.id, Some(library))
                .await?;
        }

        Ok(new_node)
    }

    /// Updates node owner, group, permissions, symlink, protection, and file data.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn save_node(&self, node: &Node) -> Result<()> {
        self.node_rows.save_node(node).await
    }

    /// Deletes `node` and its descendants. Returns false for the root folder.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn remove_node(&self, node: &Node) -> Result<bool> {
        if node.parent.is_none() {
            return Ok(false);
        }

        self.node_rows.remove_node(node.computer, node.id).await?;

        Ok(true)
    }

    /// Moves `node` and descendants into `folder` as `name`, replacing a same-named entry. Matches
    /// Grey Hack copy-then-delete behavior, so new rows are created. Returns the moved node.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn move_node(&self, node: &Node, folder: &Node, name: &str) -> Result<Node> {
        let copy = self.copy_node(node, folder, name).await?;

        if self.node_rows.node(node.computer, node.id).await?.is_some() {
            self.node_rows.remove_node(node.computer, node.id).await?;
        }

        Ok(copy)
    }
}

/// File creation, writing, copying, moving, and deletion.
pub struct FileChangeService {
    pub(crate) world: WorldRef,
    pub(crate) node_rows: Rc<NodeRepository>,
    pub(crate) library_rows: Rc<LibraryIdentityRepository>,
    pub(crate) lookup: Dep<FileLookup>,
    pub(crate) permissions: Dep<FilePermissionService>,
    pub(crate) sizes: Dep<FileSizeService>,
    pub(crate) library_files: Dep<LibraryFileService>,
    pub(crate) computer_rows: Rc<ComputerRepository>,
    pub(crate) users: Dep<UserService>,
}

impl FileChangeService {
    pub(crate) fn new(
        registry: &Rc<greyhack_world_modules::Registry>,
        node_rows: Rc<NodeRepository>,
        library_rows: Rc<LibraryIdentityRepository>,
        computer_rows: Rc<ComputerRepository>,
    ) -> Self {
        Self {
            world: registry.world::<WorldRef>().clone(),
            node_rows,
            library_rows,
            lookup: registry.get(),
            permissions: registry.get(),
            sizes: registry.get(),
            library_files: registry.get(),
            computer_rows,
            users: registry.get(),
        }
    }
}
