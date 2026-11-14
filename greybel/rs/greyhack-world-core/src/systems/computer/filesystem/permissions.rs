//! File traversal, access, permission, and ownership operations.

use std::rc::Rc;

use crate::storage::*;
use crate::systems::{FilePermissions as FilePermissionsService, *};
use crate::*;

/// Checks ancestor traversal with the lookup service shared by both permission callers.
pub(crate) async fn can_access(
    node: &Node,
    user: Option<&User>,
    lookup: &FileLookup,
) -> Result<bool> {
    let Some(user) = user else {
        return Ok(false);
    };
    let mut parent = node.parent;

    while let Some(id) = parent {
        let Some(folder) = lookup.node(node.computer, id).await? else {
            return Ok(false);
        };

        if !folder.can_execute(Some(user)) {
            return Ok(false);
        }

        parent = folder.parent;
    }

    Ok(true)
}

impl FilePermissionsService {
    /// Returns whether `user` can traverse every ancestor of `node` (`Fichero.TienePermisoAcceso`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn can_access(&self, node: &Node, user: Option<&User>) -> Result<bool> {
        can_access(node, user, &self.lookup).await
    }

    /// Returns whether `user` has `permission` on `node` and can traverse to it.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn has_node_permission(
        &self,
        node: &Node,
        permission: greyhack_behaviour::Permission,
        user: Option<&User>,
    ) -> Result<bool> {
        if !self.can_access(node, user).await? {
            return Ok(false);
        }

        Ok(match permission {
            greyhack_behaviour::Permission::Read => node.can_read(user),
            greyhack_behaviour::Permission::Write => node.can_write(user),
            greyhack_behaviour::Permission::Execute => node.can_execute(user),
        })
    }

    /// Returns whether the user named `user` on `computer` has `permission` on `node` and can traverse
    /// to it. A missing user has no permission.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn user_has_permission(
        &self,
        computer: ComputerId,
        user: &str,
        node: &Node,
        permission: greyhack_behaviour::Permission,
    ) -> Result<bool> {
        let user = self.users.user(computer, user).await?;

        self.has_node_permission(node, permission, user.as_ref())
            .await
    }

    /// Returns whether folder contents may be modified (`Carpeta.PuedeModificarContenido`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn can_modify_contents(&self, folder: &Node, user: Option<&User>) -> Result<bool> {
        Ok(self
            .has_node_permission(folder, greyhack_behaviour::Permission::Write, user)
            .await?
            && self
                .has_node_permission(folder, greyhack_behaviour::Permission::Execute, user)
                .await?)
    }

    /// Applies an `UpdatePermisos` change to `node` and, with `recursive`, permitted descendants.
    ///
    /// # Errors
    ///
    /// Returns any database error; the inner error reports malformed input or a change `user`
    /// cannot make on `node`.
    pub async fn change_permissions(
        &self,
        node: &Node,
        change: &str,
        user: Option<&User>,
        recursive: bool,
    ) -> Result<Result<(), FileFailure>> {
        let Some(permissions) = node.permissions.with_change(change) else {
            return Ok(Err(FileFailure::WrongFormat));
        };

        if !self.can_access(node, user).await? {
            return Ok(Err(FileFailure::NoPermission));
        }

        let mut copy = node.clone();

        if let Err(failure) = copy.try_set_permissions(permissions, user) {
            return Ok(Err(failure));
        }

        self.node_rows.save_node(&copy).await?;

        if recursive {
            for mut below in self.node_rows.tree_below(node.computer, &node.path).await? {
                let Some(permissions) = below.permissions.with_change(change) else {
                    continue;
                };

                if self.can_access(&below, user).await?
                    && below.try_set_permissions(permissions, user).is_ok()
                {
                    self.node_rows.save_node(&below).await?;
                }
            }
        }

        Ok(Ok(()))
    }

    /// Changes `node` owner or group (`SetOwner` / `SetGrupo`) and, with `recursive`, all
    /// descendants.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_owner(
        &self,
        node: &Node,
        owner: Option<&str>,
        group: Option<&str>,
        user: Option<&User>,
        recursive: bool,
    ) -> Result<Result<(), FileFailure>> {
        let mut nodes = vec![node.clone()];

        if recursive {
            nodes.extend(self.node_rows.tree_below(node.computer, &node.path).await?);
        }

        for (index, mut node) in nodes.into_iter().enumerate() {
            let allowed = self.can_access(&node, user).await?
                && user.is_some_and(|user| {
                    (user.is_root() || user.name == node.owner)
                        && !node.protected
                        && owner.is_none_or(|owner| user.is_root() || owner == node.owner)
                        && group.is_none_or(|group| {
                            user.is_root()
                                || group == node.group
                                || user.groups.iter().any(|candidate| candidate == group)
                        })
                });

            if !allowed {
                if index == 0 {
                    return Ok(Err(FileFailure::NoPermission));
                }

                continue;
            }

            if let Some(owner) = owner {
                owner.clone_into(&mut node.owner);
            }

            if let Some(group) = group {
                group.clone_into(&mut node.group);
            }

            self.node_rows.save_node(&node).await?;
        }

        Ok(Ok(()))
    }
}

/// File ownership and permission rules.
pub struct FilePermissions {
    pub(crate) node_rows: Rc<NodeRepository>,
    pub(crate) users: Dep<UserService>,
    pub(crate) lookup: Dep<FileLookup>,
}

impl FilePermissions {
    pub(crate) fn new(
        registry: &Rc<greyhack_world_modules::Registry>,
        node_rows: Rc<NodeRepository>,
    ) -> Self {
        Self {
            node_rows,
            users: registry.get(),
            lookup: registry.get(),
        }
    }
}
