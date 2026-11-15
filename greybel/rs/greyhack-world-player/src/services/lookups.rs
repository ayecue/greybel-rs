//! Player-script lookups (`PlayerIntrinsics`): routers and switches, directory changes, libraries,
//! account fields, mail login, and CTF accounts.

use greyhack_behaviour::{
    DeviceKind, FileKind, LibraryClass, Permission, Refusal, check_ctf_password, is_lan_ip,
    is_valid_ip, verify_password,
};
use greyhack_world_core::{ComputerId, Login, Result, RouterId};

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

/// Methods of [`World`] for player lookups.
impl Player {
    pub async fn network_router(
        &self,
        computer: ComputerId,
        terminal_computers: &[ComputerId],
        address: &str,
    ) -> Result<Option<RouterId>> {
        if !self
            .networks
            .reach
            .is_network_active(computer, terminal_computers)
            .await?
        {
            return Ok(None);
        }

        if address.is_empty() {
            Ok(self
                .networks
                .topology
                .computer(computer)
                .await?
                .and_then(|computer| computer.router))
        } else if !is_valid_ip(address) {
            Ok(None)
        } else if is_lan_ip(address) {
            self.lan_device(computer, address, &[DeviceKind::Router, DeviceKind::Switch])
                .await
        } else {
            self.network_generation.router_or_generate(address).await
        }
    }

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

    pub async fn change_folder(
        &self,
        computer: ComputerId,
        user: &str,
        current: &str,
        path: &str,
    ) -> Result<Option<FolderChange>> {
        if self.networks.topology.computer(computer).await?.is_none() {
            return Ok(None);
        }

        let user = self.computers.users.user(computer, user).await?;
        let target = match self.files.lookup.folder(computer, path, true).await? {
            None => FolderLookup::Missing,
            Some(folder)
                if !self
                    .files
                    .permissions
                    .can_access(&folder, user.as_ref())
                    .await?
                    || !self
                        .files
                        .permissions
                        .has_node_permission(&folder, Permission::Execute, user.as_ref())
                        .await? =>
            {
                FolderLookup::Denial
            }
            Some(folder) => FolderLookup::Folder(folder.path),
        };
        let current_exists = self
            .files
            .lookup
            .folder(computer, current, true)
            .await?
            .is_some();

        Ok(Some(FolderChange {
            current_exists,
            target,
        }))
    }

    pub async fn included_library(
        &self,
        computer: ComputerId,
        path: &str,
    ) -> Result<Option<IncludedLibrary>> {
        if self.networks.topology.computer(computer).await?.is_none() {
            return Ok(None);
        }

        let Some(node) = self.files.lookup.file(computer, path, true).await? else {
            return Ok(None);
        };

        if !node
            .data()
            .is_some_and(|data| data.kind == FileKind::Library)
        {
            return Ok(None);
        }

        let Some(library) = self.generator.full_library_of(&node).await? else {
            return Ok(None);
        };

        Ok(LibraryClass::of(library.kind).map(|class| IncludedLibrary {
            class,
            version: library.version,
        }))
    }

    pub async fn player_account(
        &self,
        computer: ComputerId,
        account: PlayerAccount,
    ) -> Result<Option<String>> {
        if !self.world.is_player(computer) {
            return Ok(None);
        }

        let Some(person) = self.npcs.person(self.world.player_person()).await? else {
            return Ok(None);
        };
        let value = match account {
            PlayerAccount::MailAddress => self.mail.mail_address(person.id).await?,
            PlayerAccount::BankNumber => self.bank.bank_number(person.id).await?,
        };

        Ok(value.filter(|account| !account.is_empty()))
    }

    pub async fn mail_login_from(
        &self,
        computer: ComputerId,
        terminal_computers: &[ComputerId],
        address: &str,
        password: &str,
    ) -> Result<Option<Login>> {
        if !self
            .networks
            .reach
            .is_network_active(computer, terminal_computers)
            .await?
        {
            return Ok(None);
        }

        Ok(Some(self.mail.mail_login(address, password).await?))
    }

    pub async fn ctf_login(&self, event: &str, user: &str, password: &str) -> Result<bool> {
        Ok(self.ctf.ctf_event(event).await?.is_some_and(|event| {
            event.user == user && verify_password(&event.password_hash, password)
        }))
    }

    pub async fn change_ctf_password(
        &self,
        computer: ComputerId,
        password: &str,
    ) -> Result<std::result::Result<(), CtfPasswordFailure>> {
        if self.ctf.ctf_account(computer).await?.is_none() {
            return Ok(Err(CtfPasswordFailure::NoAccount));
        }

        if let Err(refusal) = check_ctf_password(password) {
            return Ok(Err(CtfPasswordFailure::Refusal(refusal)));
        }

        self.ctf.reset_ctf_password(computer, password).await?;

        Ok(Ok(()))
    }
}

impl Player {
    pub async fn lan_device(
        &self,
        computer: ComputerId,
        address: &str,
        kinds: &[DeviceKind],
    ) -> Result<Option<RouterId>> {
        let Some(network) = self
            .networks
            .topology
            .computer(computer)
            .await?
            .map(|computer| computer.network)
        else {
            return Ok(None);
        };
        let Some(lan) = self.networks.lan.lan(network).await? else {
            return Ok(None);
        };

        Ok(lan
            .find(address)
            .map(|node| lan.node(node))
            .filter(|node| kinds.contains(&node.kind))
            .map(|node| RouterId::of(node.key)))
    }
}

/// A world service with only its declared dependencies.
pub struct Player {
    pub(crate) world: greyhack_world_core::WorldRef,
    pub(crate) computers: greyhack_world_core::Dep<greyhack_world_core::ComputerService>,
    pub(crate) files: greyhack_world_core::Dep<greyhack_world_core::FileSystemService>,
    pub(crate) networks: greyhack_world_core::Dep<greyhack_world_core::NetworkService>,
    pub(crate) network_generation:
        greyhack_world_core::Dep<greyhack_world_creation::NetworkGeneration>,
    pub(crate) generator: greyhack_world_core::Dep<greyhack_world_libraries::Generator>,
    pub(crate) mail: greyhack_world_core::Dep<greyhack_world_npcs::Mail>,
    pub(crate) npcs: greyhack_world_core::Dep<greyhack_world_npcs::NpcService>,
    pub(crate) bank: greyhack_world_core::Dep<greyhack_world_economy::Bank>,
    pub(crate) ctf: greyhack_world_core::Dep<crate::Ctf>,
}

impl Player {
    pub(crate) fn new(registry: &std::rc::Rc<greyhack_world_modules::Registry>) -> Self {
        Self {
            world: registry.world::<greyhack_world_core::WorldRef>().clone(),
            computers: registry.get(),
            files: registry.get(),
            networks: registry.get(),
            network_generation: registry.get(),
            generator: registry.get(),
            mail: registry.get(),
            npcs: registry.get(),
            bank: registry.get(),
            ctf: registry.get(),
        }
    }
}
