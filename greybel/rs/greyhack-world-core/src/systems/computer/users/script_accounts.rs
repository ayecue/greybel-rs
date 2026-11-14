//! What scripts do to the accounts of a computer through `ComputerIntrinsics`: passwords, groups, and
//! creating and deleting users.

use greyhack_behaviour::{ROOT_USER, check_new_group, check_new_user, check_password};

use crate::systems::UserService as UsersService;
use crate::systems::computer::failure::ComputerFailure;
use crate::{ComputerId, Result};

impl UsersService {
    /// Changes `user`'s password as `actor` (`ComputerIntrinsics.change_password`). Returns `None` if
    /// `computer` is gone.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn change_user_password(
        &self,
        computer: ComputerId,
        actor: &str,
        user: &str,
        password: &str,
    ) -> Result<Option<Result<(), ComputerFailure>>> {
        if self.computer_rows.computer(computer).await?.is_none() {
            return Ok(None);
        }

        if actor != ROOT_USER {
            return Ok(Some(Err(ComputerFailure::RootOnly)));
        }

        if let Err(refusal) = check_password(password) {
            return Ok(Some(Err(ComputerFailure::Refusal(refusal))));
        }

        if self.user(computer, user).await?.is_none() {
            return Ok(Some(Err(ComputerFailure::UnknownUser(user.to_owned()))));
        }

        self.set_password(computer, user, password).await?;

        Ok(Some(Ok(())))
    }

    /// Returns the groups of `name` (`ComputerIntrinsics.groups`), or `None` if `computer` is gone.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn user_groups(
        &self,
        computer: ComputerId,
        name: &str,
    ) -> Result<Option<Result<Vec<String>, ComputerFailure>>> {
        if self.computer_rows.computer(computer).await?.is_none() {
            return Ok(None);
        }

        Ok(Some(match self.user(computer, name).await? {
            None => Err(ComputerFailure::UnknownUser(name.to_owned())),
            Some(user) => Ok(user.groups),
        }))
    }

    /// Adds `user` to `group` as `actor` (`ComputerIntrinsics.create_group`). Returns `None` if
    /// `computer` is gone.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn add_user_to_group(
        &self,
        computer: ComputerId,
        actor: &str,
        user: &str,
        group: &str,
    ) -> Result<Option<Result<(), ComputerFailure>>> {
        if self.computer_rows.computer(computer).await?.is_none() {
            return Ok(None);
        }

        if actor != ROOT_USER {
            return Ok(Some(Err(ComputerFailure::RootOnly)));
        }

        if let Err(refusal) = check_new_group(user, group) {
            return Ok(Some(Err(ComputerFailure::Refusal(refusal))));
        }

        if self.user(computer, user).await?.is_none() {
            return Ok(Some(Err(ComputerFailure::UnknownUser(user.to_owned()))));
        }

        self.add_group(computer, user, group).await?;

        Ok(Some(Ok(())))
    }

    /// Removes `user` from `group` as `actor` (`ComputerIntrinsics.delete_group`). Returns `None` if
    /// `computer` is gone.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn remove_user_from_group(
        &self,
        computer: ComputerId,
        actor: &str,
        user: &str,
        group: &str,
    ) -> Result<Option<Result<(), ComputerFailure>>> {
        if self.computer_rows.computer(computer).await?.is_none() {
            return Ok(None);
        }

        if actor != ROOT_USER {
            return Ok(Some(Err(ComputerFailure::RootOnly)));
        }

        if self.user(computer, user).await?.is_none() {
            return Ok(Some(Err(ComputerFailure::UnknownUser(user.to_owned()))));
        }

        Ok(Some(if self.remove_group(computer, user, group).await? {
            Ok(())
        } else {
            Err(ComputerFailure::UnknownGroup {
                group: group.to_owned(),
                user: user.to_owned(),
            })
        }))
    }

    /// Creates `user` with `password` as `actor` (`ComputerIntrinsics.create_user`). Returns `None`
    /// if `computer` is gone.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn create_user_account(
        &self,
        computer: ComputerId,
        actor: &str,
        user: &str,
        password: &str,
    ) -> Result<Option<Result<(), ComputerFailure>>> {
        if self.computer_rows.computer(computer).await?.is_none() {
            return Ok(None);
        }

        if actor != ROOT_USER {
            return Ok(Some(Err(ComputerFailure::RootOnly)));
        }

        if let Err(refusal) = check_new_user(user, password) {
            return Ok(Some(Err(ComputerFailure::Refusal(refusal))));
        }

        Ok(Some(
            self.add_user(computer, user, password)
                .await?
                .map_err(ComputerFailure::User),
        ))
    }

    /// Deletes `name` as `actor`, with its home folder when `remove_home` is set
    /// (`ComputerIntrinsics.delete_user`). Returns `None` if `computer` is gone.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn delete_user_account(
        &self,
        computer: ComputerId,
        actor: &str,
        name: &str,
        remove_home: bool,
    ) -> Result<Option<Result<(), ComputerFailure>>> {
        if self.computer_rows.computer(computer).await?.is_none() {
            return Ok(None);
        }

        if actor != ROOT_USER {
            return Ok(Some(Err(ComputerFailure::RootOnly)));
        }

        match self.user(computer, name).await? {
            None => return Ok(Some(Err(ComputerFailure::UnknownUser(name.to_owned())))),
            Some(user) if user.is_root() => {
                return Ok(Some(Err(ComputerFailure::RootDeletion)));
            }
            Some(user) if !user.deletable => {
                return Ok(Some(Err(ComputerFailure::UndeletableUser)));
            }
            Some(_) => {}
        }

        self.remove_user(computer, name, remove_home).await?;

        Ok(Some(Ok(())))
    }
}
