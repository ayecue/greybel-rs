//! What the core code needs from the domains above it.
//!
//! Computers and networks must act on state that higher domains own: the state of an NPC, the trace
//! of the player, sniffed packets, networks generated on first contact. Core cannot call those
//! domains, so each such call belongs to a small hook trait that the world installs when it is
//! built ([`crate::World::on`]). A hook runs exactly where the call it replaces ran, so the order
//! of its side effects and random draws does not change.
//!
//! World futures are single-threaded, so hooks return futures that are not `Send`. The hooks
//! themselves are `Send + Sync`, so the world stays `Send`: tests and hosts move it between
//! threads.

use std::future::Future;
use std::pin::Pin;

use greyhack_behaviour::LogAction;

use crate::{ComputerId, NetworkId, Node, NodeDraft, PersonId, Result, RouterId, WorldRef};

/// The future of a hook.
pub type HookFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T>> + 'a>>;

/// Which NPC [`NpcHook::npc_state`] describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcLookup<'a> {
    /// The NPC that is `person`.
    Person(PersonId),
    /// The NPC that owns account `user` on `computer`.
    Account {
        /// Computer with the account.
        computer: ComputerId,
        /// Account name.
        user: &'a str,
    },
}

/// Owned login data published to packet-capture subscribers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginPacket {
    /// Computer the login comes from.
    pub source: ComputerId,
    /// Computer that was logged in to.
    pub destination: ComputerId,
    /// Address the login connected to.
    pub address: String,
    /// Port the login connected to.
    pub port: u16,
    /// User of the login.
    pub user: String,
    /// Password of the login.
    pub password: String,
}

impl greyhack_world_modules::Event for LoginPacket {}

/// A deletion that may start an administrator's active trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceAction {
    /// Computer whose file was deleted.
    pub remote: ComputerId,
    /// Number of bounces used by the deleting program.
    pub bounces: u32,
    /// Action written to the system log.
    pub action: LogAction,
    /// Terminal associated with the action.
    pub terminal: u64,
}

impl greyhack_world_modules::Event for TraceAction {}

/// How an NPC stands at the world's current second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcState {
    /// Whether the NPC is dead (`NpcInfo.IsDead`).
    pub dead: bool,
    /// Whether the NPC is dead or fired from its job.
    pub dead_or_fired: bool,
    /// Whether the NPC is logged in (`Persona.IsLogged`).
    pub online: bool,
    /// Whether the NPC is at work by its schedule.
    pub at_work: bool,
}

/// Account-file operations used by `UserService` while file services depend on `UserService`.
///
/// `CoreModule::new` registers the `FileSystems` implementation in the caller's registry, breaking the
/// construction cycle without a back-pointer to the module or its repository aggregate.
pub(crate) trait AccountFiles {
    /// Returns the computer's root folder.
    fn root(&self, computer: ComputerId) -> HookFuture<'_, Option<Node>>;

    /// Returns a file by path.
    fn file<'a>(
        &'a self,
        computer: ComputerId,
        path: &'a str,
        follow: bool,
    ) -> HookFuture<'a, Option<Node>>;

    /// Returns a folder by path.
    fn folder<'a>(
        &'a self,
        computer: ComputerId,
        path: &'a str,
        follow: bool,
    ) -> HookFuture<'a, Option<Node>>;

    /// Returns a folder's direct subfolders.
    fn folders<'a>(&'a self, folder: &'a Node) -> HookFuture<'a, Vec<Node>>;

    /// Adds an account file or folder.
    fn add_node<'a>(&'a self, folder: &'a Node, node: &'a NodeDraft) -> HookFuture<'a, Node>;

    /// Removes an account file or folder.
    fn remove_node<'a>(&'a self, node: &'a Node) -> HookFuture<'a, bool>;

    /// Saves an account file or folder.
    fn save_node<'a>(&'a self, node: &'a Node) -> HookFuture<'a, ()>;
}

/// Generates networks reached by core.
pub trait NetworkGeneration {
    /// Returns the main router of the network at `public_ip`, generating the network when the
    /// address belongs to one that was not made yet. Higher domain: creation.
    fn router_or_generate<'a>(
        &'a self,
        world: &'a WorldRef,
        public_ip: &'a str,
    ) -> HookFuture<'a, Option<RouterId>>;
}

/// Looks up NPC state for core networks.
pub trait NpcHook {
    /// Returns how the NPC `lookup` stands, or `None` when there is no such NPC. Higher domain:
    /// npcs.
    fn npc_state<'a>(
        &'a self,
        world: &'a WorldRef,
        lookup: NpcLookup<'a>,
    ) -> HookFuture<'a, Option<NpcState>>;
}

/// Player traces triggered by core actions.
pub trait PlayerTraces {
    /// Returns the token the player's log entries carry now. Higher domain: guilds.
    fn player_token_trace<'a>(&'a self, world: &'a WorldRef) -> HookFuture<'a, String>;
}

/// Library changes requested by core.
pub trait LibraryVersions {
    /// Changes a library file's version while preserving its zones and weaknesses. Higher domain:
    /// libraries (`RedGlobal.ShouldRestore`).
    fn set_library_version<'a>(
        &'a self,
        world: &'a WorldRef,
        computer: ComputerId,
        path: &'a str,
        version: &'a str,
    ) -> HookFuture<'a, bool> {
        let _ = (world, computer, path, version);

        Box::pin(async { Ok(false) })
    }
}

/// Network-administrator lookup for core accounts.
pub trait NetworkAdminHook {
    /// Returns the person administering `network`, if any. Higher domain: npcs.
    fn network_admin_person<'a>(
        &'a self,
        world: &'a WorldRef,
        network: NetworkId,
    ) -> HookFuture<'a, Option<PersonId>> {
        let _ = (world, network);

        Box::pin(async { Ok(None) })
    }
}

/// Domain state persisted with core metadata.
pub trait PersistentState {
    /// Loads domain-owned persisted state after core loads `world_meta`. Higher domain: smart
    /// devices for the traffic cooldown.
    fn load_state<'a>(&'a self, world: &'a WorldRef) -> HookFuture<'a, ()> {
        let _ = world;

        Box::pin(async { Ok(()) })
    }

    /// Writes domain-owned persisted state when core commits `world_meta`. Higher domain: smart
    /// devices for the traffic cooldown.
    fn save_state<'a>(&'a self, world: &'a WorldRef) -> HookFuture<'a, ()> {
        let _ = world;

        Box::pin(async { Ok(()) })
    }
}
