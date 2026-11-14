//! Permission-filtered tree deletion matching Grey Hack.

use crate::systems::FileChanges as FileChangesService;
use crate::{FileFailure, Node, Result, User};

impl FileChangesService {
    async fn delete_entries(&self, node: &Node, user: Option<&User>) -> Result<(bool, bool)> {
        let Some(parent_id) = node.parent else {
            return Ok((false, false));
        };

        if node.is_protected_in_root() {
            return Ok((false, false));
        }

        let mut any_success = false;

        if node.is_folder() && !node.is_symlink() {
            let children = self.lookup.children(node).await?;

            if !children.is_empty()
                && (!self
                    .permissions
                    .has_node_permission(node, greyhack_behaviour::Permission::Read, user)
                    .await?
                    || !self
                        .permissions
                        .has_node_permission(node, greyhack_behaviour::Permission::Execute, user)
                        .await?)
            {
                return Ok((false, false));
            }

            for child in children {
                let (child_success, _) = Box::pin(self.delete_entries(&child, user)).await?;

                any_success |= child_success;
            }

            if !self.lookup.children(node).await?.is_empty() {
                return Ok((any_success, false));
            }
        }

        let parent = self.lookup.node(node.computer, parent_id).await?;

        if let Some(parent) = parent
            && self.permissions.can_modify_contents(&parent, user).await?
        {
            self.node_rows.remove_node(node.computer, node.id).await?;

            return Ok((true, true));
        }

        Ok((any_success, false))
    }

    /// Deletes permitted entries at and below `node` (`FileSystem.RemoveFicheroRecursivo`). The
    /// successful boolean reports skipped entries.
    ///
    /// # Errors
    ///
    /// Returns any database error; the inner error explains why deletion stopped before changing
    /// anything.
    pub async fn delete_with_rules(
        &self,
        node: &Node,
        user: Option<&User>,
    ) -> Result<Result<bool, FileFailure>> {
        if !self.permissions.can_access(node, user).await? {
            return Ok(Err(FileFailure::DeletePermission(node.path.clone())));
        }

        let (any_success, node_gone) = self.delete_entries(node, user).await?;

        if !any_success {
            return Ok(Err(FileFailure::DeletePermission(node.path.clone())));
        }

        Ok(Ok(!node_gone))
    }
}
