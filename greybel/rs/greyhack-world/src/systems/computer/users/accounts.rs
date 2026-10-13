//! Computer accounts, passwords, groups, and `/etc/passwd` management.

use greyhack_behaviour::{
    GUEST_USER, PASSWD_FOLDER, PASSWD_PATH, PermissionSet, ROOT_USER, check_account_capacity,
    check_account_name,
};

use crate::model::{etc_folder, home_folders, passwd_file_draft, passwd_is_reusable, passwd_text};
use crate::{ComputerId, NodeDraft, Result, User, UserFailure, World};

impl World {
    /// Returns `computer` users in creation order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn users(&self, computer: ComputerId) -> Result<Vec<User>> {
        self.db.users(computer).await
    }

    /// Returns the first non-root username used by normal-user exploits.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn normal_user(&self, computer: ComputerId) -> Result<Option<String>> {
        Ok(self
            .db
            .users(computer)
            .await?
            .into_iter()
            .find(|user| user.name != ROOT_USER)
            .map(|user| user.name))
    }

    /// Validates `password` for `user` on `computer` (`Computer.ValidateUserPass`). On computers
    /// neither owned by nor created for the player, also requires a living, employed NPC: the
    /// network administrator for root, otherwise the account-owning NPC.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn check_login(
        &self,
        computer: ComputerId,
        user: &str,
        password: &str,
    ) -> Result<Login> {
        if !self.check_password(computer, user, password).await? {
            return Ok(Login::InvalidCredentials);
        }

        let Some(row) = self.db.computer(computer).await? else {
            return Ok(Login::InvalidCredentials);
        };

        if self.is_player(computer) || row.network_kind.is_player_made() {
            return Ok(Login::Valid);
        }

        let npc = if user == ROOT_USER {
            match self.db.network_admin_person(row.network).await? {
                Some(person) => self.db.npc(person).await?,
                None => None,
            }
        } else {
            self.npc_on_computer(computer, user).await?
        };

        Ok(
            if npc.is_some_and(|npc| npc.is_dead_or_fired(self.time_seconds)) {
                Login::AccountInactive
            } else {
                Login::Valid
            },
        )
    }

    /// Returns whether `password` matches `user` on `computer`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn check_password(
        &self,
        computer: ComputerId,
        user: &str,
        password: &str,
    ) -> Result<bool> {
        Ok(self
            .db
            .user(computer, user)
            .await?
            .is_some_and(|user| user.password == password))
    }

    /// Creates a user with home folder and `/etc/passwd` entry, or returns an error for
    /// duplicate/reserved names or the user limit.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn add_user(
        &mut self,
        computer: ComputerId,
        name: &str,
        password: &str,
    ) -> Result<Result<(), UserFailure>> {
        if let Err(refusal) = check_account_name(name) {
            return Ok(Err(refusal.into()));
        }

        if self.user(computer, name).await?.is_some() {
            return Ok(Err(UserFailure::UserExists(name.to_owned())));
        }

        if let Err(refusal) = check_account_capacity(self.db.user_count(computer).await?) {
            return Ok(Err(refusal.into()));
        }

        let mut user = User::new(name, password);

        user.deletable = true;
        self.db.insert_user(computer, &user).await?;

        let Some(root) = self.root(computer).await? else {
            return Ok(Ok(()));
        };
        let home = match self.folder(computer, "/home", true).await? {
            Some(home) => home,
            None => {
                self.add_node(
                    &root,
                    &NodeDraft::folder("home", ROOT_USER, PermissionSet::default_for(true)),
                )
                .await?
            }
        };
        let folder = self
            .add_node(
                &home,
                &NodeDraft::folder(name, name, PermissionSet::default_for(true)),
            )
            .await?;

        for child in home_folders(name, PermissionSet::GROUP_FOLDER) {
            self.add_node(&folder, &child).await?;
        }

        self.write_passwd(computer, false).await?;

        Ok(Ok(()))
    }

    /// Deletes user `name` from `computer` and optionally its home with `remove_home`
    /// (`DeleteUser`). Returns whether it existed.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn remove_user(
        &mut self,
        computer: ComputerId,
        name: &str,
        remove_home: bool,
    ) -> Result<bool> {
        if !self.db.remove_user(computer, name).await? {
            return Ok(false);
        }

        if remove_home && let Some(home) = self.folder(computer, "/home", true).await? {
            let folder = self
                .folders(&home)
                .await?
                .into_iter()
                .find(|folder| folder.name() == name);

            if let Some(folder) = folder {
                self.remove_node(&folder).await?;
            }
        }

        self.write_passwd(computer, false).await?;

        Ok(true)
    }

    /// Changes the password for `name` on `computer` and regenerates `/etc/passwd`.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn set_password(
        &mut self,
        computer: ComputerId,
        name: &str,
        password: &str,
    ) -> Result<()> {
        let Some(mut user) = self.db.user(computer, name).await? else {
            return Ok(());
        };

        user.set_password(password);
        self.db.save_user(computer, &user).await?;

        self.write_passwd(computer, true).await
    }

    /// Writes one `name:md5` line per user to `/etc/passwd`. Creates missing or binary files;
    /// `recreate` always replaces the file.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn write_passwd(&mut self, computer: ComputerId, recreate: bool) -> Result<()> {
        let users = self.db.users(computer).await?;
        let content = passwd_text(&users);
        let passwd = self.file(computer, PASSWD_PATH, true).await?;
        let mut file = match passwd {
            Some(file) if passwd_is_reusable(file.data(), recreate) => file,
            _ => {
                if let Some(file) = passwd {
                    self.remove_node(&file).await?;
                }

                let Some(root) = self.root(computer).await? else {
                    return Ok(());
                };
                let etc = match self.folder(computer, PASSWD_FOLDER, true).await? {
                    Some(etc) => etc,
                    None => self.add_node(&root, &etc_folder()).await?,
                };

                self.add_node(&etc, &passwd_file_draft()).await?
            }
        };

        if let Some(data) = file.data_mut() {
            data.set_content(&content);
        }

        file.timestamp = self.time_seconds;

        self.save_node(&file).await
    }

    /// Returns user `name` on `computer`, including the implicit `guest` account.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn user(&self, computer: ComputerId, name: &str) -> Result<Option<User>> {
        if name == GUEST_USER {
            return Ok(Some(User::guest()));
        }

        self.db.user(computer, name).await
    }

    /// Adds `group` to `user` on `computer`. Returns whether the user exists and lacked the group.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn add_group(
        &mut self,
        computer: ComputerId,
        user: &str,
        group: &str,
    ) -> Result<bool> {
        self.db.add_group(computer, user, group).await
    }

    /// Removes `group` from `user` on `computer`. Returns whether the user had it.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn remove_group(
        &mut self,
        computer: ComputerId,
        user: &str,
        group: &str,
    ) -> Result<bool> {
        self.db.remove_group(computer, user, group).await
    }

    /// Registers `plain` independently of an account and returns its MD5 (`Database.AddPassword`).
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn register_password(&mut self, plain: &str) -> Result<String> {
        self.db.register_password(plain).await
    }

    /// Returns plaintext for MD5 `hash` if previously registered (`decipher`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn password_of_hash(&self, hash: &str) -> Result<Option<String>> {
        self.db.plain_password(hash).await
    }
}

/// Credential validation result (`Computer.ValidateUserPass`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Login {
    Valid,
    /// The username or password is invalid.
    InvalidCredentials,
    /// The password is valid, but the account NPC is dead or fired.
    AccountInactive,
}
