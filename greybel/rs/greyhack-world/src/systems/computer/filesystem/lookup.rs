//! Node and path lookups: symlink-aware paths, folders, files, and children.

use greyhack_behaviour::MAX_SYMLINKS;
use greyhack_behaviour::path::{self, normalize};

use crate::{ComputerId, Node, NodeId, Result, World};

impl World {
    /// Returns computer node `id`, or `None` if removed.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn node(&self, computer: ComputerId, id: NodeId) -> Result<Option<Node>> {
        self.db.node(computer, id).await
    }

    /// Returns the computer root folder.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn root(&self, computer: ComputerId) -> Result<Option<Node>> {
        self.db.root(computer).await
    }

    /// Returns folder `path` using `GetCarpeta`; follows the final symlink when `follow` is set.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn folder(
        &self,
        computer: ComputerId,
        path: &str,
        follow: bool,
    ) -> Result<Option<Node>> {
        if path.is_empty() {
            return Ok(None);
        }

        let mut path = path.to_owned();
        let mut follow = follow;

        for links in 0.. {
            let clean_path = normalize(&path);
            let Some(node) = self
                .db
                .node_at(computer, clean_path.as_ref())
                .await?
                .filter(Node::is_folder)
            else {
                return Ok(None);
            };

            if !follow || !node.is_symlink() {
                return Ok(Some(node));
            }

            if node.symlink == node.path || links > MAX_SYMLINKS {
                return Ok(None);
            }

            if Box::pin(self.folder(computer, &node.symlink, false))
                .await?
                .is_none()
            {
                return Ok(Some(node));
            }

            path.clone_from(&node.symlink);
            follow = true;
        }

        Ok(None)
    }

    /// Returns file `path` using `GetArchivo`. Resolves symlinks in parent folders and follows the
    /// final symlink when `follow` is set.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn file(
        &self,
        computer: ComputerId,
        path: &str,
        follow: bool,
    ) -> Result<Option<Node>> {
        if path.is_empty() {
            return Ok(None);
        }

        let mut path = normalize(path).into_owned();
        let mut follow = follow;

        for links in 0.. {
            // A direct full-path match proves no parent symlink, so parent traversal is needed only
            // after a miss through a symlinked folder.
            let node = match self.db.node_at(computer, &path).await? {
                Some(node) if !node.is_folder() => Some(node),
                Some(_) => None,
                None => {
                    let Some(folder) = self.folder(computer, path::parent(&path), true).await?
                    else {
                        return Ok(None);
                    };

                    self.db
                        .node_at(computer, &path::join(&folder.path, path::file_name(&path)))
                        .await?
                        .filter(|node| !node.is_folder())
                }
            };
            let Some(node) = node else {
                return Ok(None);
            };

            if !follow || !node.is_symlink() {
                return Ok(Some(node));
            }

            if node.symlink == node.path || links > MAX_SYMLINKS {
                return Ok(None);
            }

            if Box::pin(self.file(computer, &node.symlink, false))
                .await?
                .is_none()
            {
                return Ok(Some(node));
            }

            path.clone_from(&node.symlink);
            follow = true;
        }

        Ok(None)
    }

    /// Returns the file or folder at `path` (`GetFichero`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn entry(
        &self,
        computer: ComputerId,
        path: &str,
        follow: bool,
    ) -> Result<Option<Node>> {
        match self.file(computer, path, follow).await? {
            Some(file) => Ok(Some(file)),
            None => self.folder(computer, path, follow).await,
        }
    }

    /// Returns the entry at `path` if `user` exists and can traverse every parent
    /// (`HelperScript.HelperValFile.GetAccessibleFile`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn entry_for_user(
        &self,
        computer: ComputerId,
        path: &str,
        follow: bool,
        user: &str,
    ) -> Result<Option<Node>> {
        let Some(account) = self.user(computer, user).await? else {
            return Ok(None);
        };
        let Some(node) = self.entry(computer, path, follow).await? else {
            return Ok(None);
        };

        Ok(self
            .can_access(&node, Some(&account))
            .await?
            .then_some(node))
    }

    /// Returns contents of file `path`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn content(
        &self,
        computer: ComputerId,
        path: &str,
        follow: bool,
    ) -> Result<Option<String>> {
        Ok(self
            .file(computer, path, follow)
            .await?
            .and_then(|node| node.data().map(|data| data.content.clone())))
    }

    /// Returns folder children, folders before files, each group in insertion order
    /// (`GetFicheros`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn children(&self, folder: &Node) -> Result<Vec<Node>> {
        self.db.children(folder.computer, folder.id).await
    }

    /// Returns `folder` subfolders in insertion order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn folders(&self, folder: &Node) -> Result<Vec<Node>> {
        let mut children = self.children(folder).await?;

        children.retain(Node::is_folder);

        Ok(children)
    }

    /// Returns `folder` files in insertion order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn files(&self, folder: &Node) -> Result<Vec<Node>> {
        let mut children = self.children(folder).await?;

        children.retain(|node| !node.is_folder());

        Ok(children)
    }

    /// Returns child `name` of `folder`, whether file or folder.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn child(&self, folder: &Node, name: &str) -> Result<Option<Node>> {
        self.db
            .node_at(folder.computer, &path::join(&folder.path, name))
            .await
    }
}
