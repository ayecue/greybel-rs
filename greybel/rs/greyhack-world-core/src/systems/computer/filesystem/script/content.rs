//! File reads and writes by scripts (`FileIntrinsics`).

use greyhack_behaviour::{FileData, Permission, check_content};

use super::failure::ScriptFileFailure;
use crate::systems::ScriptFileService as ScriptFilesService;
use crate::{FileFailure, Node, Result};

impl ScriptFilesService {
    /// Lists the subfolders or files of folder `node` for `user` (`FileIntrinsics.get_folders`,
    /// `get_files`). `path` is the path the script used.
    ///
    /// # Errors
    ///
    /// Returns any database read error; the inner error reports a folder `user` cannot read.
    pub async fn list_entries(
        &self,
        node: &Node,
        user: &str,
        path: &str,
        folders: bool,
    ) -> Result<Result<Vec<Node>, FileFailure>> {
        let account = self.users.user(node.computer, user).await?;

        if !self
            .permissions
            .has_node_permission(node, Permission::Read, account.as_ref())
            .await?
        {
            return Ok(Err(FileFailure::FolderReadPermission(path.to_owned())));
        }

        Ok(Ok(if folders {
            self.lookup.folders(node).await?
        } else {
            self.lookup.files(node).await?
        }))
    }

    /// Returns the text of file `node` for `user` (`FileIntrinsics.get_content`). `path` is the path
    /// the script used.
    ///
    /// # Errors
    ///
    /// Returns any database read error; the inner error reports why the file cannot be read.
    pub async fn read_content(
        &self,
        node: &Node,
        user: &str,
        path: &str,
    ) -> Result<Result<String, ScriptFileFailure>> {
        let Some(data) = node.data().filter(|data| !data.is_binary()) else {
            return Ok(Err(ScriptFileFailure::Binary(path.to_owned())));
        };
        let account = self.users.user(node.computer, user).await?;

        if !node.can_read(account.as_ref()) {
            return Ok(Err(ScriptFileFailure::NoReadPermission(path.to_owned())));
        }

        Ok(Ok(data.content.clone()))
    }

    /// Replaces the text of file `node` as `user` and updates its timestamp
    /// (`FileIntrinsics.set_content`). `path` is the path the script used. Returns whether `user` may
    /// write.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error; the inner error reports invalid content or a binary
    /// file.
    pub async fn write_content(
        &self,
        node: &Node,
        user: &str,
        path: &str,
        content: &str,
    ) -> Result<Result<bool, ScriptFileFailure>> {
        let mut node = node.clone();

        if let Err(refusal) = check_content(content) {
            return Ok(Err(refusal.into()));
        }

        if node.data().is_none_or(FileData::is_binary) {
            return Ok(Err(ScriptFileFailure::Binary(path.to_owned())));
        }

        let account = self.users.user(node.computer, user).await?;

        if !self
            .permissions
            .has_node_permission(&node, Permission::Write, account.as_ref())
            .await?
        {
            return Ok(Ok(false));
        }

        if let Some(data) = node.data_mut() {
            data.set_content(content);
        }

        node.timestamp = self.world.time_seconds();
        self.changes.save_node(&node).await?;

        Ok(Ok(true))
    }
}
