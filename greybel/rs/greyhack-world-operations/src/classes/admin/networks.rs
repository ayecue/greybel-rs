//! Network and program creation (`admin.so`).

use greyhack_behaviour::{FileData, PermissionSet, path};
use greyhack_world::{ComputerId, NodeDraft, RouterId, World};

use crate::Result;
use crate::protocol::{AdminFailure, NetworkOptions};

/// Created router or computer ID, or the creation failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Creation<T> {
    Object(T),
    Failure(AdminFailure),
}

/// Creates a network if neither its public address nor domain is already registered.
///
/// # Errors
///
/// Returns errors from database reads.
pub(super) async fn create_network(
    world: &mut World,
    network: NetworkOptions,
) -> Result<Creation<RouterId>> {
    if let Some(address) = &network.public_ip
        && world.router_at(address).await?.is_some()
    {
        return Ok(Creation::Failure(AdminFailure::AddressTaken(
            address.clone(),
        )));
    }

    if let Some(domain) = &network.domain
        && world.lookup(domain).await?.is_some()
    {
        return Ok(Creation::Failure(AdminFailure::DomainTaken(domain.clone())));
    }

    let router = world
        .add_network(&greyhack_world::NetworkOptions {
            public_ip: network.public_ip,
            essid: network.essid,
            wifi_password: network.wifi_password,
            domain: network.domain,
            position: None,
            computers: network.computers,
        })
        .await?;

    Ok(Creation::Object(router))
}

/// Creates script `source` at `path` on `computer` with owner `user`.
///
/// # Errors
///
/// Returns an error when the world database refuses a read.
pub(super) async fn create_program(
    world: &mut World,
    computer: ComputerId,
    user: &str,
    path: &str,
    source: &str,
) -> Result<bool> {
    let Some(folder) = world.folder(computer, path::parent(path), true).await? else {
        return Ok(false);
    };
    let node = NodeDraft::file(
        path::file_name(path),
        user,
        PermissionSet::default_for(false),
        FileData::script(source),
    );

    world.add_node(&folder, &node).await?;

    Ok(true)
}
