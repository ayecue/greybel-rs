//! Copies, moves, symlinks, renames, and deletions by scripts (`FileIntrinsics`,
//! `PlayerUtils.MoverFichero`, `PlayerUtils.RemoveFile`).

use greyhack_behaviour::{
    FileData, FileKind, LogAction, MAX_FILES, Permission, PermissionSet, check_new_name,
    check_target_name, path,
};

use super::failure::ScriptFileFailure;
use crate::{FileFailure, LogWrite, Node, NodeDraft, Result, World};

/// A completed script copy.
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptCopy {
    /// The copy, or the existing folder merged with the source.
    pub node: Node,
    /// Some entries were skipped.
    pub skipped: bool,
}

/// Where a script deletion happened, for the log and the trace it starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeletionOrigin<'a> {
    /// Address the script came to the computer from.
    pub prev_ip: &'a str,
    /// Path the script used for the file.
    pub path: &'a str,
    /// Number of bounces between the script and the computer.
    pub bounces: u32,
    /// Terminal of the deleting program.
    pub terminal: u64,
}

impl World {
    /// Validates and copies `node` to `folder`/`name` as `user` with `PlayerUtils.MoverFichero` rules
    /// (`FileIntrinsics.copy`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error; failures are in the inner result.
    pub async fn copy_entry(
        &mut self,
        node: &Node,
        user: &str,
        folder: &str,
        name: &str,
    ) -> Result<Result<ScriptCopy, ScriptFileFailure>> {
        if let Err(refusal) = check_target_name(name) {
            return Ok(Err(refusal.into()));
        }

        if !path::is_valid(folder) {
            return Ok(Err(FileFailure::InvalidPath.into()));
        }

        let Some(destination) = self.folder(node.computer, folder, true).await? else {
            return Ok(Err(FileFailure::MissingFolder(folder.to_owned()).into()));
        };
        let account = self.user(node.computer, user).await?;

        Ok(
            match self
                .copy_with_rules(
                    node,
                    &destination,
                    name,
                    account.as_ref(),
                    account.as_ref(),
                    false,
                )
                .await?
            {
                Ok(report) => Ok(ScriptCopy {
                    node: report.node,
                    skipped: report.skipped,
                }),
                Err(failure) => Err(failure.into()),
            },
        )
    }

    /// Moves `node` to `folder`/`name` as `user` with `PlayerUtils.MoverFichero` rules
    /// (`FileIntrinsics.move`) and returns the moved node.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error; failures are in the inner result.
    pub async fn move_entry(
        &mut self,
        node: &Node,
        user: &str,
        folder: &str,
        name: &str,
    ) -> Result<Result<Node, ScriptFileFailure>> {
        if let Err(refusal) = check_target_name(name) {
            return Ok(Err(refusal.into()));
        }

        if path::has_too_many_folders(folder) {
            return Ok(Err(ScriptFileFailure::TooManyPathFolders));
        }

        if !path::is_valid(folder) {
            return Ok(Err(FileFailure::InvalidPath.into()));
        }

        let computer = node.computer;
        let Some(destination) = self.folder(computer, folder, true).await? else {
            return Ok(Err(FileFailure::MissingFolder(folder.to_owned()).into()));
        };
        let account = self.user(computer, user).await?;

        if node.path == path::ROOT {
            return Ok(Err(ScriptFileFailure::RootMove));
        }

        if !self.can_access(&destination, account.as_ref()).await?
            || !self
                .can_modify_contents(&destination, account.as_ref())
                .await?
        {
            return Ok(Err(
                FileFailure::CannotWrite(destination.path.clone()).into()
            ));
        }

        if !self.can_access(node, account.as_ref()).await? {
            return Ok(Err(FileFailure::CannotAccess(node.path.clone()).into()));
        }

        let parent = match node.parent {
            Some(parent) => self.node(computer, parent).await?,
            None => None,
        };
        let can_move_from_parent = match &parent {
            Some(parent) => self.can_modify_contents(parent, account.as_ref()).await?,
            None => false,
        };

        if !can_move_from_parent || node.is_protected_in_root() {
            return Ok(Err(FileFailure::NoPermission.into()));
        }

        if node.is_folder()
            && !node.is_symlink()
            && parent
                .as_ref()
                .is_some_and(|parent| parent.id != destination.id)
            && !self
                .has_node_permission(node, Permission::Write, account.as_ref())
                .await?
        {
            return Ok(Err(FileFailure::NoPermission.into()));
        }

        if let Some(same_name_node) = self.child(&destination, name).await? {
            if same_name_node.is_protected_in_root() {
                return Ok(Err(
                    FileFailure::SystemFileTarget(node.name().to_owned()).into()
                ));
            }

            if same_name_node.is_folder() != node.is_folder() {
                return Ok(Err(FileFailure::KindMismatch.into()));
            }
        }

        Ok(Ok(self.move_node(node, &destination, name).await?))
    }

    /// Creates `folder`/`name` as a symlink to `target` as `user` (`FileIntrinsics.symlink`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error; failures are in the inner result.
    pub async fn symlink_entry(
        &mut self,
        target: &Node,
        user: &str,
        folder: &str,
        name: &str,
    ) -> Result<Result<(), ScriptFileFailure>> {
        if let Err(refusal) = check_target_name(name) {
            return Ok(Err(refusal.into()));
        }

        let computer = target.computer;
        let target_path = target.path.clone();

        if path::has_too_many_folders(folder) {
            return Ok(Err(ScriptFileFailure::TooManyPathFolders));
        }

        if !path::is_valid(folder) {
            return Ok(Err(FileFailure::InvalidPath.into()));
        }

        if folder == target_path {
            return Ok(Err(ScriptFileFailure::SymlinkToItself));
        }

        let Some(destination) = self.folder(computer, folder, true).await? else {
            return Ok(Err(FileFailure::MissingFolder(folder.to_owned()).into()));
        };

        if self
            .entry(computer, &path::join(folder, name), true)
            .await?
            .is_some()
        {
            return Ok(Err(ScriptFileFailure::Exists(name.to_owned())));
        }

        if target.is_folder() {
            return self
                .create_folder_in(computer, user, folder, name, &target_path)
                .await;
        }

        if self.file_count(computer).await? > MAX_FILES {
            return Ok(Err(FileFailure::TooManyFiles.into()));
        }

        let account = self.user(computer, user).await?;

        if !destination.can_write(account.as_ref()) {
            return Ok(Err(ScriptFileFailure::NoCreatePermission {
                folder: folder.to_owned(),
                name: name.to_owned(),
            }));
        }

        let data = target.data().cloned().unwrap_or_default();
        let mut link = NodeDraft::file(
            name,
            user,
            PermissionSet::default_for(false),
            FileData {
                kind: if data.is_binary() {
                    FileKind::Binary
                } else {
                    FileKind::Text
                },
                command: data.command,
                command_name: data.command_name.clone(),
                ..FileData::default()
            },
        );

        link.symlink = target_path;
        self.add_node(&destination, &link).await?;

        Ok(Ok(()))
    }

    /// Renames `node` as `user` and returns the renamed node (`FileIntrinsics.rename`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error; failures are in the inner result.
    pub async fn rename_entry(
        &mut self,
        node: &Node,
        user: &str,
        name: &str,
    ) -> Result<Result<Node, ScriptFileFailure>> {
        if let Err(refusal) = check_new_name(name) {
            return Ok(Err(refusal.into()));
        }

        if node.path == path::ROOT || node.is_protected_in_root() {
            return Ok(Err(FileFailure::SystemFile.into()));
        }

        let computer = node.computer;
        let parent = node.parent.expect("not the root");
        let Some(parent) = self.node(computer, parent).await? else {
            return Ok(Err(FileFailure::NoPermission.into()));
        };
        let account = self.user(computer, user).await?;

        if !self.can_modify_contents(&parent, account.as_ref()).await? {
            return Ok(Err(FileFailure::NoPermission.into()));
        }

        if self.child(&parent, name).await?.is_some() {
            return Ok(Err(ScriptFileFailure::NameExists));
        }

        Ok(Ok(self.move_node(node, &parent, name).await?))
    }

    /// Deletes `node` as `user` and, once something was removed, logs the deletion on its computer
    /// and starts an active trace on its network (`FileIntrinsics.delete`,
    /// `PlayerUtils.RemoveFile`). Returns whether entries were skipped.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error; failures are in the inner result.
    pub async fn delete_entry(
        &mut self,
        node: &Node,
        user: &str,
        origin: &DeletionOrigin<'_>,
    ) -> Result<Result<bool, ScriptFileFailure>> {
        let computer = node.computer;
        let account = self.user(computer, user).await?;
        let partial = match self.delete_with_rules(node, account.as_ref()).await? {
            Ok(partial) => partial,
            Err(failure) => return Ok(Err(failure.into())),
        };

        self.write_log(
            computer,
            &LogWrite {
                action: LogAction::Remove,
                ip: origin.prev_ip,
                port: 0,
                file: origin.path,
                bounce_ip: "",
            },
        )
        .await?;
        self.trace_action(computer, origin.bounces, LogAction::Remove, origin.terminal)
            .await?;

        Ok(Ok(partial))
    }
}
