//! File and folder creation by scripts (`PlayerUtils.CrearArchivo`, `PlayerUtils.CrearCarpeta`).

use greyhack_behaviour::{
    FileData, MAX_FILES, MAX_FILES_PER_FOLDER, PermissionSet, check_file_name, check_folder_name,
    file_size_mb, path,
};

use super::failure::ScriptFileFailure;
use crate::systems::ScriptFileService as ScriptFilesService;
use crate::{ComputerId, FileFailure, NodeDraft, Result};

impl ScriptFilesService {
    /// Creates file `name` with `data` under `folder_path` as `user` with `PlayerUtils.CrearArchivo`
    /// rules.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error; creation failures are in the inner result.
    pub async fn create_file_in(
        &self,
        computer: ComputerId,
        user: &str,
        folder_path: &str,
        name: &str,
        data: FileData,
    ) -> Result<Result<(), ScriptFileFailure>> {
        if self.sizes.file_count(computer).await? > MAX_FILES {
            return Ok(Err(FileFailure::TooManyFiles.into()));
        }

        if !path::is_valid(folder_path) {
            return Ok(Err(FileFailure::InvalidPath.into()));
        }

        let Some(folder) = self.lookup.folder(computer, folder_path, true).await? else {
            return Ok(Err(
                FileFailure::MissingFolder(folder_path.to_owned()).into()
            ));
        };
        let account = self.users.user(computer, user).await?;

        if !self
            .permissions
            .can_modify_contents(&folder, account.as_ref())
            .await?
        {
            return Ok(Err(ScriptFileFailure::NoCreatePermission {
                folder: folder_path.to_owned(),
                name: name.to_owned(),
            }));
        }

        if file_size_mb(data.size) > self.sizes.available_storage_mb(computer).await? {
            return Ok(Err(FileFailure::DiskFull.into()));
        }

        if self.lookup.children(&folder).await?.len() > MAX_FILES_PER_FOLDER {
            return Ok(Err(FileFailure::FolderFull.into()));
        }

        let node = NodeDraft::file(name, user, PermissionSet::default_for(false), data);

        self.changes.add_node(&folder, &node).await?;

        Ok(Ok(()))
    }

    /// Creates an empty text file `name` under `folder` as `user` (`ComputerIntrinsics.touch`).
    /// Returns `None` if `computer` no longer exists.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error; failures are in the inner result.
    pub async fn touch_file(
        &self,
        computer: ComputerId,
        user: &str,
        folder: &str,
        name: &str,
    ) -> Result<Option<Result<(), ScriptFileFailure>>> {
        if self.computer_rows.computer(computer).await?.is_none() {
            return Ok(None);
        }

        if let Err(refusal) = check_file_name(name) {
            return Ok(Some(Err(refusal.into())));
        }

        if self
            .lookup
            .entry(computer, &path::join(folder, name), true)
            .await?
            .is_some()
        {
            return Ok(Some(Err(ScriptFileFailure::Exists(name.to_owned()))));
        }

        Ok(Some(
            self.create_file_in(computer, user, folder, name, FileData::text(""))
                .await?,
        ))
    }

    /// Creates user-owned directory `name` under `folder_path` as `user`, or a symlink to `symlink`
    /// when it is nonempty.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error; creation failures are in the inner result.
    pub async fn create_folder_in(
        &self,
        computer: ComputerId,
        user: &str,
        folder_path: &str,
        name: &str,
        symlink: &str,
    ) -> Result<Result<(), ScriptFileFailure>> {
        if folder_path.is_empty() {
            return Ok(Err(ScriptFileFailure::EmptyPath));
        }

        if let Err(refusal) = check_folder_name(name) {
            return Ok(Err(refusal.into()));
        }

        let Some(folder) = self.lookup.folder(computer, folder_path, true).await? else {
            return Ok(Err(
                FileFailure::MissingFolder(folder_path.to_owned()).into()
            ));
        };
        let account = self.users.user(computer, user).await?;

        if !self
            .permissions
            .can_modify_contents(&folder, account.as_ref())
            .await?
        {
            return Ok(Err(FileFailure::NoPermission.into()));
        }

        let new_path = path::join(&folder.path, name);

        if self
            .lookup
            .entry(computer, &new_path, true)
            .await?
            .is_some()
        {
            return Ok(Err(ScriptFileFailure::Exists(name.to_owned())));
        }

        if self.sizes.file_count(computer).await? > MAX_FILES {
            return Ok(Err(ScriptFileFailure::TooManyFolders));
        }

        if self.lookup.children(&folder).await?.len() > MAX_FILES_PER_FOLDER {
            return Ok(Err(FileFailure::FolderFull.into()));
        }

        if path::has_too_many_folders(&new_path) {
            return Ok(Err(ScriptFileFailure::TooManySubfolders));
        }

        let mut node = NodeDraft::folder(name, user, PermissionSet::default_for(true));

        symlink.clone_into(&mut node.symlink);
        self.changes.add_node(&folder, &node).await?;

        Ok(Ok(()))
    }

    /// Creates a directory `name` under `folder` as `user` (`ComputerIntrinsics.create_folder`).
    /// Returns `None` if `computer` no longer exists.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error; failures are in the inner result.
    pub async fn create_folder(
        &self,
        computer: ComputerId,
        user: &str,
        folder: &str,
        name: &str,
    ) -> Result<Option<Result<(), ScriptFileFailure>>> {
        if self.computer_rows.computer(computer).await?.is_none() {
            return Ok(None);
        }

        Ok(Some(
            self.create_folder_in(computer, user, folder, name, "")
                .await?,
        ))
    }
}
