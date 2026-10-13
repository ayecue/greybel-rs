//! Script-object classes and their operation handlers. Handlers translate program messages into world
//! calls and convert world results into protocol responses.

pub mod admin;
pub mod admin_computer;
pub mod admin_network;
pub mod apt_client;
pub mod blockchain;
pub mod coin;
pub mod computer;
pub mod crypto;
pub mod ctf;
pub mod debug_library;
pub mod file;
pub mod meta_lib;
pub mod meta_mail;
pub mod metaxploit;
pub mod net_session;
pub mod player;
pub mod port;
pub mod router;
pub mod service;
pub mod shell;
pub mod smart_appliance;
pub mod sub_wallet;
pub mod traffic;
pub mod wallet;

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greyhack_world::{Computer, ComputerId, Node, World};

pub(crate) use self::computer::creation_outcome;
use crate::Result;
use crate::connection::Connection;
use crate::connection::objects::{Access, FileRef};
use crate::protocol::{GameServer, ScriptFailure};

/// Looks up the computer referenced by `access`.
pub(crate) async fn computer(world: &World, access: &Access) -> Result<Option<Computer>> {
    Ok(world.computer(access.computer).await?)
}

/// Looks up the node for `file`, returning `None` after removal.
pub(crate) async fn node(world: &World, file: &FileRef) -> Result<Option<Node>> {
    Ok(world.node(file.access.computer, file.node).await?)
}

/// Requires network access from the player's computer for terminals in `terminal_computers`
/// (`Computer.IsNetworkActive` of `GreyInterpreter.GetPlayerComputer`); otherwise returns no connection.
pub(crate) async fn require_connection(
    world: &World,
    terminal_computers: &[ComputerId],
) -> Result<()> {
    if world
        .is_network_active(world.player(), terminal_computers)
        .await?
    {
        Ok(())
    } else {
        Err(ScriptFailure::NoConnection.into())
    }
}

/// Registers all class handlers for `connection` on `channel`.
pub(crate) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    admin::attach(connection, channel);
    apt_client::attach(connection, channel);
    blockchain::attach(connection, channel);
    coin::attach(connection, channel);
    computer::attach(connection, channel);
    crypto::attach(connection, channel);
    ctf::attach(connection, channel);
    debug_library::attach(connection, channel);
    file::attach(connection, channel);
    meta_lib::attach(connection, channel);
    meta_mail::attach(connection, channel);
    metaxploit::attach(connection, channel);
    net_session::attach(connection, channel);
    player::attach(connection, channel);
    router::attach(connection, channel);
    service::attach(connection, channel);
    shell::attach(connection, channel);
    smart_appliance::attach(connection, channel);
    sub_wallet::attach(connection, channel);
    traffic::attach(connection, channel);
    wallet::attach(connection, channel);
    admin_network::attach(connection, channel);
    admin_computer::attach(connection, channel);
}
