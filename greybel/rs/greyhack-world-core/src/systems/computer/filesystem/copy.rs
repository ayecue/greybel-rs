//! Grey Hack-compatible tree copy validation, filtering, accounting, and cross-computer copies.

use greyhack_behaviour::{BYTES_PER_MB, MAX_FILES, MAX_FILES_PER_FOLDER, path};

use crate::model::computer::filesystem::CopyPlan;
use crate::storage::nodes::NodeSpec;
use crate::systems::FileChangeService as FileChangesService;
use crate::{ComputerId, CopyReport, FileFailure, Node, Result, User};

impl FileChangesService {
    async fn validate_replacement(
        &self,
        node: &Node,
        user: Option<&User>,
    ) -> Result<Option<String>> {
        let Some(parent_id) = node.parent else {
            return Ok(Some(node.path.clone()));
        };
        let parent = self.lookup.node(node.computer, parent_id).await?;

        if !parent.is_some_and(|parent| parent.can_write(user) && parent.can_execute(user))
            || node.is_protected_in_root()
            || !self.permissions.can_access(node, user).await?
        {
            return Ok(Some(node.path.clone()));
        }

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
                return Ok(Some(node.path.clone()));
            }

            for child in children {
                if let Some(path) = Box::pin(self.validate_replacement(&child, user)).await? {
                    return Ok(Some(path));
                }
            }
        }

        Ok(None)
    }

    /// Implements `FileSystem.ValidarDestinoCopia` for a resolved parent and destination.
    async fn validate_copy_entry(
        &self,
        source: &Node,
        parent: &Node,
        destination: Option<&Node>,
        user: Option<&User>,
    ) -> Result<Result<(), FileFailure>> {
        if !parent.can_execute(user)
            || parent.protected
            || !self.permissions.can_access(parent, user).await?
        {
            return Ok(Err(FileFailure::CannotWrite(parent.path.clone())));
        }

        let Some(destination) = destination else {
            if !self.permissions.can_modify_contents(parent, user).await? {
                return Ok(Err(FileFailure::CannotWrite(parent.path.clone())));
            }

            return Ok(
                if self.lookup.children(parent).await?.len() < MAX_FILES_PER_FOLDER {
                    Ok(())
                } else {
                    Err(FileFailure::FolderFull)
                },
            );
        };

        if destination.is_protected_in_root() {
            return Ok(Err(FileFailure::SystemFileTarget(source.name().to_owned())));
        }

        if source.computer == destination.computer && source.id == destination.id {
            return Ok(Err(FileFailure::OverwriteSelf));
        }

        if source.is_folder() != destination.is_folder() {
            return Ok(Err(FileFailure::KindMismatch));
        }

        let merge = source.is_folder()
            && !source.is_symlink()
            && destination.is_folder()
            && !destination.is_symlink();

        if merge {
            return Ok(
                if destination.can_execute(user)
                    && !destination.protected
                    && self.permissions.can_access(destination, user).await?
                {
                    Ok(())
                } else {
                    Err(FileFailure::CannotWrite(destination.path.clone()))
                },
            );
        }

        if !destination.is_folder() && !destination.is_symlink() && !source.is_symlink() {
            return Ok(
                if self
                    .permissions
                    .has_node_permission(destination, greyhack_behaviour::Permission::Write, user)
                    .await?
                {
                    Ok(())
                } else {
                    Err(FileFailure::CannotWrite(destination.path.clone()))
                },
            );
        }

        Ok(match self.validate_replacement(destination, user).await? {
            Some(path) => Err(FileFailure::CannotReplace(path)),
            None => Ok(()),
        })
    }

    /// Computes the copyable subset and resource accounting without modifying the file system.
    async fn inspect_copy(
        &self,
        source: &Node,
        parent: Option<&Node>,
        destination: Option<&Node>,
        source_user: Option<&User>,
        destination_user: Option<&User>,
        top: bool,
    ) -> Result<Result<CopyPlan, FileFailure>> {
        let readable = self
            .permissions
            .has_node_permission(source, greyhack_behaviour::Permission::Read, source_user)
            .await?;
        let executable = !source.is_folder()
            || self
                .permissions
                .has_node_permission(source, greyhack_behaviour::Permission::Execute, source_user)
                .await?;

        if !readable || !executable {
            return Ok(if top {
                Err(FileFailure::CannotRead(source.path.clone()))
            } else {
                Ok(CopyPlan {
                    skipped: true,
                    ..CopyPlan::default()
                })
            });
        }

        if let Some(parent) = parent
            && let Err(failure) = self
                .validate_copy_entry(source, parent, destination, destination_user)
                .await?
        {
            return Ok(if top {
                Err(failure)
            } else {
                Ok(CopyPlan {
                    skipped: true,
                    ..CopyPlan::default()
                })
            });
        }

        let mut plan = CopyPlan {
            new_entries: usize::from(destination.is_none()),
            ..CopyPlan::default()
        };

        if !source.is_folder() || source.is_symlink() {
            plan.file_size = source.own_size(false);

            return Ok(Ok(plan));
        }

        let merge = destination
            .is_some_and(|destination| destination.is_folder() && !destination.is_symlink());
        let children = self.lookup.children(source).await?;
        let mut available = if let Some(destination) = destination.filter(|_| merge) {
            MAX_FILES_PER_FOLDER.saturating_sub(self.lookup.children(destination).await?.len())
        } else {
            usize::MAX
        };

        for child in children {
            let destination_node = if let Some(destination) = destination.filter(|_| merge) {
                self.lookup.child(destination, child.name()).await?
            } else {
                None
            };

            if merge && destination_node.is_none() && available == 0 {
                plan.skipped = true;
                continue;
            }

            let child_plan = match Box::pin(self.inspect_copy(
                &child,
                destination.filter(|_| merge),
                destination_node.as_ref(),
                source_user,
                destination_user,
                false,
            ))
            .await?
            {
                Ok(plan) => plan,
                Err(failure) => return Ok(Err(failure)),
            };

            plan.new_entries += child_plan.new_entries;
            plan.file_size += child_plan.file_size;
            plan.skipped |= child_plan.skipped;
            if destination_node.is_none() && child_plan.new_entries != 0 {
                available = available.saturating_sub(1);
            }
        }

        Ok(Ok(plan))
    }

    async fn copy_single(
        &self,
        source: &Node,
        folder: &Node,
        name: &str,
        destination_user: Option<&str>,
    ) -> Result<Node> {
        let library = self.library_files.library_of(source).await?;
        let destination_node = self.lookup.child(folder, name).await?;

        if let Some(destination_node) = &destination_node {
            self.node_rows
                .remove_node(folder.computer, destination_node.id)
                .await?;
        }

        let mut fields = NodeSpec::of(source);

        if let Some(user) = destination_user {
            fields.owner = user;
            fields.group = user;
        }

        if let Some(destination_node) = &destination_node {
            let preserve = if destination_user.is_some() {
                destination_node.is_folder() == source.is_folder()
            } else {
                !destination_node.is_folder() && !destination_node.is_symlink()
            };

            if preserve {
                fields.owner = &destination_node.owner;
                fields.group = &destination_node.group;
                fields.permissions = destination_node.permissions;
            }
        }

        let id = self
            .node_rows
            .insert_node(
                folder.computer,
                Some((folder.id, &folder.path)),
                name,
                &fields,
            )
            .await?;
        let copy = self
            .node_rows
            .node(folder.computer, id)
            .await?
            .expect("the node just inserted");

        if let Some(library) = &library {
            self.library_files
                .set_library_of(&copy, Some(library))
                .await?;
        }

        Ok(copy)
    }

    async fn copy_branch(
        &self,
        source: &Node,
        folder: &Node,
        name: &str,
        source_user: Option<&User>,
        destination_user: Option<&User>,
        destination_account: Option<&str>,
    ) -> Result<Node> {
        let destination_node = self.lookup.child(folder, name).await?;
        let merge = source.is_folder()
            && !source.is_symlink()
            && destination_node
                .as_ref()
                .is_some_and(|node| node.is_folder() && !node.is_symlink());
        let copy = if merge {
            destination_node.clone().expect("a merge destination")
        } else {
            self.copy_single(source, folder, name, destination_account)
                .await?
        };

        if source.is_folder() && !source.is_symlink() {
            for child in self.lookup.children(source).await? {
                let allowed = self
                    .permissions
                    .has_node_permission(&child, greyhack_behaviour::Permission::Read, source_user)
                    .await?
                    && (!child.is_folder()
                        || self
                            .permissions
                            .has_node_permission(
                                &child,
                                greyhack_behaviour::Permission::Execute,
                                source_user,
                            )
                            .await?);
                let destination = self.lookup.child(&copy, child.name()).await?;
                let within_limit = destination.is_some()
                    || self.lookup.children(&copy).await?.len() < MAX_FILES_PER_FOLDER;
                let valid = !merge
                    || self
                        .validate_copy_entry(&child, &copy, destination.as_ref(), destination_user)
                        .await?
                        .is_ok();

                if allowed && within_limit && valid {
                    Box::pin(self.copy_branch(
                        &child,
                        &copy,
                        child.name(),
                        source_user,
                        destination_user,
                        destination_account,
                    ))
                    .await?;
                }
            }
        }

        Ok(copy)
    }

    /// Validates, accounts for, and performs a filtered copy (`PlayerUtils.MoverFichero`,
    /// `PrepararScpUpload`).
    ///
    /// # Errors
    ///
    /// Returns any database error; the inner error explains why no entries can be copied.
    #[expect(
        clippy::cast_precision_loss,
        reason = "PlayerUtils.PrepararScpUpload compares byte counts using C# float megabytes"
    )]
    pub async fn copy_with_rules(
        &self,
        source: &Node,
        folder: &Node,
        name: &str,
        source_user: Option<&User>,
        destination_user: Option<&User>,
        transfer_ownership: bool,
    ) -> Result<Result<CopyReport, FileFailure>> {
        if !self
            .permissions
            .can_access(folder, destination_user)
            .await?
        {
            return Ok(Err(FileFailure::CannotWrite(folder.path.clone())));
        }

        let destination_node = self.lookup.child(folder, name).await?;

        if let Err(failure) = self
            .validate_copy_entry(source, folder, destination_node.as_ref(), destination_user)
            .await?
        {
            return Ok(Err(failure));
        }

        if !self.permissions.can_access(source, source_user).await? {
            return Ok(Err(FileFailure::CannotAccess(source.path.clone())));
        }

        let plan = match self
            .inspect_copy(
                source,
                Some(folder),
                destination_node.as_ref(),
                source_user,
                destination_user,
                true,
            )
            .await?
        {
            Ok(plan) => plan,
            Err(failure) => return Ok(Err(failure)),
        };

        if plan.file_size as f64 / BYTES_PER_MB
            > self.sizes.available_storage_mb(folder.computer).await?
        {
            return Ok(Err(FileFailure::DiskFull));
        }

        if self.sizes.file_count(folder.computer).await? + plan.new_entries > MAX_FILES {
            return Ok(Err(FileFailure::TooManyFiles));
        }

        let node = self
            .copy_branch(
                source,
                folder,
                name,
                source_user,
                destination_user,
                if transfer_ownership {
                    destination_user.map(|user| user.name.as_str())
                } else {
                    None
                },
            )
            .await?;

        Ok(Ok(CopyReport {
            node,
            skipped: plan.skipped,
        }))
    }

    /// Copies `node` and descendants into `folder` on any computer as `name`, replacing a
    /// same-named entry. Preserves source ownership and timestamps (`FileSystem.CopiarFichero`). Copying a
    /// folder into itself uses its original state. Returns the stored copy.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn copy_node(&self, node: &Node, folder: &Node, name: &str) -> Result<Node> {
        let below = self.node_rows.tree_below(node.computer, &node.path).await?;
        let mut libraries = Vec::with_capacity(below.len() + 1);

        for original in std::iter::once(node).chain(&below) {
            libraries.push(self.library_files.library_of(original).await?);
        }

        if let Some(same_name) = self.lookup.child(folder, name).await? {
            self.node_rows
                .remove_node(folder.computer, same_name.id)
                .await?;
        }

        let mut copies: Vec<(String, Node)> = Vec::with_capacity(below.len() + 1);

        for (original, library) in std::iter::once(node).chain(&below).zip(libraries) {
            let (parent, copy_name) = match copies.last() {
                None => (folder.clone(), name.to_owned()),
                Some(_) => {
                    let parent_path = path::parent(&original.path);
                    let parent = copies
                        .iter()
                        .find(|(path, _)| path == parent_path)
                        .map(|(_, copy)| copy.clone())
                        .expect("a parent copied before its children");

                    (parent, original.name().to_owned())
                }
            };
            let fields = NodeSpec::of(original);

            let id = self
                .node_rows
                .insert_node(
                    folder.computer,
                    Some((parent.id, &parent.path)),
                    &copy_name,
                    &fields,
                )
                .await?;
            let copy = self
                .node_rows
                .node(folder.computer, id)
                .await?
                .expect("the node just inserted");

            if let Some(library) = &library {
                self.library_files
                    .set_library_of(&copy, Some(library))
                    .await?;
            }

            copies.push((original.path.clone(), copy));
        }

        Ok(copies.swap_remove(0).1)
    }

    /// Copies `source` from `from_computer` as `from_user` into `folder` on `to_computer` as
    /// `to_user`, with `scp` and `ShellIntrinsics.scp` checks.
    ///
    /// # Errors
    ///
    /// Returns any database error; the inner error explains why the copy was rejected.
    pub async fn copy_between(
        &self,
        (from_computer, from_user): (ComputerId, &str),
        source: &str,
        (to_computer, to_user): (ComputerId, &str),
        folder: &str,
    ) -> Result<Result<crate::CopyReport, FileFailure>> {
        if !self.computer_rows.has_computer(from_computer).await?
            || !self.computer_rows.has_computer(to_computer).await?
        {
            return Ok(Err(FileFailure::ComputerUnavailable));
        }

        // `ShellIntrinsics.scp` resolves source, then destination, before
        // `PlayerUtils.PrepararScpUpload`.
        let Some(entry) = self.lookup.entry(from_computer, source, false).await? else {
            return Ok(Err(FileFailure::MissingFile(source.to_owned())));
        };
        let Some(target) = self.lookup.entry(to_computer, folder, true).await? else {
            return Ok(Err(FileFailure::MissingFolder(folder.to_owned())));
        };

        if !target.is_folder() {
            return Ok(Err(FileFailure::NotAFolder(folder.to_owned())));
        }

        if source == path::ROOT {
            return Ok(Err(FileFailure::InvalidPath));
        }

        let source_account = self.users.user(from_computer, from_user).await?;
        let destination_account = self.users.user(to_computer, to_user).await?;
        let name = entry.name().to_owned();

        self.copy_with_rules(
            &entry,
            &target,
            &name,
            source_account.as_ref(),
            destination_account.as_ref(),
            true,
        )
        .await
    }
}
