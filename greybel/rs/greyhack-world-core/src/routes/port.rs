//! Port snapshots (`PortIntrinsics`). Programs read captured port data directly, so no operations are registered.

use greyhack_world_programs::protocol;

use crate::Port;

/// Converts port data to protocol form with the LAN address from its computer or forward.
#[must_use]
pub fn snapshot(port: &Port, lan_ip: &str) -> protocol::Port {
    protocol::Port {
        number: port.number,
        internal_port: port.number,
        service: port.service.name().to_owned(),
        version: port.version.clone(),
        lan_ip: lan_ip.to_owned(),
        closed: port.closed,
        visible: port.visible,
    }
}
