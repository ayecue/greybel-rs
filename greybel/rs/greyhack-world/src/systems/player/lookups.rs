//! Player-script lookups (`PlayerIntrinsics`): routers and switches, directory changes, libraries,
//! account fields, mail login, and CTF accounts.

use greyhack_behaviour::{
    DeviceKind, FileKind, LibraryClass, Permission, Refusal, check_ctf_password, is_lan_ip,
    is_valid_ip, verify_password,
};

use crate::{ComputerId, Login, Result, RouterId, World};

/// Account field of the player's person (`PlayerIntrinsics.user_mail_address`, `user_bank_number`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerAccount {
    /// Registered mail address.
    MailAddress,
    /// Registered bank account number.
    BankNumber,
}

/// Target of a `cd` (`PlayerIntrinsics.cd`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderLookup {
    /// No folder exists at the path.
    Missing,
    /// The folder exists but the user cannot enter it.
    Denial,
    /// Normalized path of the enterable folder.
    Folder(String),
}

/// Result of a `cd` (`PlayerIntrinsics.cd`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderChange {
    /// Whether the current folder still exists.
    pub current_exists: bool,
    /// What the target path resolves to.
    pub target: FolderLookup,
}

/// Library `include_lib` loads (`PlayerIntrinsics.include_lib`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludedLibrary {
    /// Script object type of the library.
    pub class: LibraryClass,
    /// Version of the library.
    pub version: String,
}

/// Why a CTF password reset failed (`PlayerIntrinsics.reset_ctf_password`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CtfPasswordFailure {
    /// The computer has no CTF account.
    NoAccount,
    /// The password breaks an argument rule.
    Refusal(Refusal),
}

impl World {
    /// Returns `computer`'s router for an empty `address`, or resolves the router at `address` and
    /// generates missing public networks. Returns `None` when `computer` has no network for
    /// `terminal_computers` (`PlayerIntrinsics.get_router`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn network_router(
        &mut self,
        computer: ComputerId,
        terminal_computers: &[ComputerId],
        address: &str,
    ) -> Result<Option<RouterId>> {
        if !self.is_network_active(computer, terminal_computers).await? {
            return Ok(None);
        }

        if address.is_empty() {
            Ok(self
                .computer(computer)
                .await?
                .and_then(|computer| computer.router))
        } else if !is_valid_ip(address) {
            Ok(None)
        } else if is_lan_ip(address) {
            self.lan_device(computer, address, &[DeviceKind::Router, DeviceKind::Switch])
                .await
        } else {
            self.router_or_generate(address).await
        }
    }

    /// Finds the switch at LAN `address` on `computer`'s network regardless of connectivity
    /// (`PlayerIntrinsics.get_switch`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn lan_switch(
        &self,
        computer: ComputerId,
        address: &str,
    ) -> Result<Option<RouterId>> {
        if !is_valid_ip(address) || !is_lan_ip(address) {
            return Ok(None);
        }

        self.lan_device(computer, address, &[DeviceKind::Switch])
            .await
    }

    /// Finds a device of `kinds` at `address` on `computer`'s network (`NetworkLan.FindDevice`,
    /// `Router.GetLanRouter`), without requiring it to respond to the computer.
    async fn lan_device(
        &self,
        computer: ComputerId,
        address: &str,
        kinds: &[DeviceKind],
    ) -> Result<Option<RouterId>> {
        let Some(network) = self
            .computer(computer)
            .await?
            .map(|computer| computer.network)
        else {
            return Ok(None);
        };
        let Some(lan) = self.lan(network).await? else {
            return Ok(None);
        };

        Ok(lan
            .find(address)
            .map(|node| lan.node(node))
            .filter(|node| kinds.contains(&node.kind))
            .map(|node| RouterId::of(node.key)))
    }

    /// Resolves a `cd` target from `current` as `user` on `computer` (`PlayerIntrinsics.cd`); returns
    /// `None` after computer removal.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn change_folder(
        &self,
        computer: ComputerId,
        user: &str,
        current: &str,
        path: &str,
    ) -> Result<Option<FolderChange>> {
        if self.computer(computer).await?.is_none() {
            return Ok(None);
        }

        let user = self.user(computer, user).await?;
        let target = match self.folder(computer, path, true).await? {
            None => FolderLookup::Missing,
            Some(folder)
                if !self.can_access(&folder, user.as_ref()).await?
                    || !self
                        .has_node_permission(&folder, Permission::Execute, user.as_ref())
                        .await? =>
            {
                FolderLookup::Denial
            }
            Some(folder) => FolderLookup::Folder(folder.path),
        };
        let current_exists = self.folder(computer, current, true).await?.is_some();

        Ok(Some(FolderChange {
            current_exists,
            target,
        }))
    }

    /// Resolves the library at `path` on `computer` as the object its library kind gives
    /// (`PlayerIntrinsics.include_lib`): the game reads the kind from the file's library content
    /// (`Library.GetIdLib`), not from its name, and follows symlinks without a permission check. Other
    /// files, and libraries of any other kind, return `None`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn included_library(
        &self,
        computer: ComputerId,
        path: &str,
    ) -> Result<Option<IncludedLibrary>> {
        if self.computer(computer).await?.is_none() {
            return Ok(None);
        }

        let Some(node) = self.file(computer, path, true).await? else {
            return Ok(None);
        };

        if !node
            .data()
            .is_some_and(|data| data.kind == FileKind::Library)
        {
            return Ok(None);
        }

        let Some(library) = self.library_of(&node).await? else {
            return Ok(None);
        };

        Ok(LibraryClass::of(library.kind).map(|class| IncludedLibrary {
            class,
            version: library.version,
        }))
    }

    /// Returns a configured account field only when `computer` belongs to the player.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn player_account(
        &self,
        computer: ComputerId,
        account: PlayerAccount,
    ) -> Result<Option<String>> {
        if !self.is_player(computer) {
            return Ok(None);
        }

        let Some(person) = self.person(self.player_person()).await? else {
            return Ok(None);
        };
        let value = match account {
            PlayerAccount::MailAddress => person.mail_address,
            PlayerAccount::BankNumber => person.bank_number,
        };

        Ok(value.filter(|account| !account.is_empty()))
    }

    /// Authenticates mail account `address` from `computer` (`PlayerIntrinsics.mail_login`). Returns
    /// `None` when `computer` has no network access for `terminal_computers`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn mail_login_from(
        &mut self,
        computer: ComputerId,
        terminal_computers: &[ComputerId],
        address: &str,
        password: &str,
    ) -> Result<Option<Login>> {
        if !self.is_network_active(computer, terminal_computers).await? {
            return Ok(None);
        }

        Ok(Some(self.mail_login(address, password).await?))
    }

    /// Validates `user` and `password` for CTF event `event` (`PlayerIntrinsics.get_ctf`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn ctf_login(&self, event: &str, user: &str, password: &str) -> Result<bool> {
        Ok(self.ctf_event(event).await?.is_some_and(|event| {
            event.user == user && verify_password(&event.password_hash, password)
        }))
    }

    /// Updates the CTF account password for `computer` (`PlayerIntrinsics.reset_ctf_password`).
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn change_ctf_password(
        &mut self,
        computer: ComputerId,
        password: &str,
    ) -> Result<std::result::Result<(), CtfPasswordFailure>> {
        if self.ctf_account(computer).await?.is_none() {
            return Ok(Err(CtfPasswordFailure::NoAccount));
        }

        if let Err(refusal) = check_ctf_password(password) {
            return Ok(Err(CtfPasswordFailure::Refusal(refusal)));
        }

        self.reset_ctf_password(computer, password).await?;

        Ok(Ok(()))
    }
}
