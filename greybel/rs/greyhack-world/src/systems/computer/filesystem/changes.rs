//! Node creation, updates, deletion, and moves.

use crate::storage::nodes::NodeSpec;
use crate::{Node, NodeDraft, Result, World};

impl World {
    /// Adds `node` to `folder`, replacing a same-named entry and storing its library. Returns the
    /// stored node.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn add_node(&mut self, folder: &Node, node: &NodeDraft) -> Result<Node> {
        if let Some(same_name) = self.child(folder, &node.name).await? {
            self.db.remove_node(folder.computer, same_name.id).await?;
        }

        let mut node = node.clone();

        // Player-created files and folders use the world clock (`PlayerUtils.CrearArchivo`,
        // `CrearCarpeta`).
        node.timestamp = self.time_seconds;

        let id = self
            .db
            .insert_node(
                folder.computer,
                Some((folder.id, &folder.path)),
                &node.name,
                &NodeSpec::of_draft(&node),
            )
            .await?;
        let new_node = self
            .db
            .node(folder.computer, id)
            .await?
            .expect("the node just inserted");

        if let Some(library) = node.library() {
            self.set_library_of(&new_node, Some(library)).await?;
        }

        Ok(new_node)
    }

    /// Updates node owner, group, permissions, symlink, protection, and file data.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn save_node(&mut self, node: &Node) -> Result<()> {
        self.db.save_node(node).await
    }

    /// Deletes `node` and its descendants. Returns false for the root folder.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn remove_node(&mut self, node: &Node) -> Result<bool> {
        if node.parent.is_none() {
            return Ok(false);
        }

        self.db.remove_node(node.computer, node.id).await?;

        Ok(true)
    }

    /// Moves `node` and descendants into `folder` as `name`, replacing a same-named entry. Matches
    /// Grey Hack copy-then-delete behavior, so new rows are created. Returns the moved node.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn move_node(&mut self, node: &Node, folder: &Node, name: &str) -> Result<Node> {
        let copy = self.copy_node(node, folder, name).await?;

        if self.db.node(node.computer, node.id).await?.is_some() {
            self.db.remove_node(node.computer, node.id).await?;
        }

        Ok(copy)
    }
}
