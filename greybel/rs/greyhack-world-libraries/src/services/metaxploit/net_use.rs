//! Opening a `metaxploit.net_use` session (`MetaExploitIntrinsics.net_use`).

use greyhack_behaviour::{FileKind, LanFailure, is_device, is_lan_ip, is_valid_ip, library_path};
use greyhack_world_core::systems::network::reach::is_closed;
use greyhack_world_core::{ComputerId, Result, RouterId, WorldError};

use crate::{Metaxploit, NetUseFailure, NetUseSession};

/// Methods of [`World`] for exploits metaxploit net use.
impl Metaxploit {
    async fn router_session(
        &self,
        lan: bool,
        address: &str,
        router: &greyhack_world_core::Router,
    ) -> Result<Option<NetUseSession>> {
        let kernel = if lan {
            let device = self
                .networks
                .lan
                .lan(router.network)
                .await?
                .and_then(|lan| {
                    lan.find(address)
                        .filter(|node| is_device(lan.node(*node).kind))
                        .map(|node| RouterId::of(lan.node(node).key))
                });
            let Some(device) = device else {
                return Ok(None);
            };

            device
        } else {
            router.id
        };

        return Ok(self
            .networks
            .routers
            .kernel_library(kernel)
            .await?
            .map(|_| NetUseSession::Router {
                router: kernel,
                address: address.to_owned(),
            }));
    }

    pub async fn net_use(
        &self,
        from: ComputerId,
        address: &str,
        port: u16,
    ) -> Result<Result<Option<NetUseSession>, NetUseFailure>> {
        if !is_valid_ip(address) {
            return Ok(Err(NetUseFailure::InvalidAddress));
        }

        let Some(computer) = self.computers.computer(from).await? else {
            return Ok(Ok(None));
        };
        let lan = is_lan_ip(address);
        let router = if lan {
            computer.router
        } else {
            self.network_generation
                .router_or_generate(&self.world, address)
                .await?
        };
        let Some(router) = (match router {
            Some(router) => self.networks.router(router).await?,
            None => None,
        }) else {
            return Ok(Err(NetUseFailure::UnknownAddress));
        };

        if !self
            .networks
            .reach
            .is_available(router.id.computer())
            .await?
        {
            return Ok(Err(NetUseFailure::MainRouterDown));
        }

        // `ScriptsUtil.IsFirewallBlocking`: computers can reach their own addresses.
        let source = if lan {
            computer.local_ip.clone()
        } else {
            self.networks.topology.public_ip(from).await?
        };
        let firewall = if source == address {
            None
        } else {
            self.networks
                .reach
                .firewall_ping_status(router.network, &source, address, port)
                .await?
        };

        if firewall == Some(LanFailure::Unreachable) {
            return Ok(Err(NetUseFailure::Lan(LanFailure::Unreachable)));
        }

        if port == 0 {
            return Ok(Ok(self.router_session(lan, address, &router).await?));
        }

        if let Some(failure) = firewall {
            return Ok(Err(NetUseFailure::Lan(failure)));
        }

        let Some(lan_tree) = self.networks.lan.lan(router.network).await? else {
            return Ok(Err(NetUseFailure::UnknownComputer));
        };
        // Resolve the computer and its listening service port.
        let (target, internal) = if lan {
            let Some(node) = lan_tree.find(address) else {
                return Ok(Err(NetUseFailure::UnknownComputer));
            };
            let node = lan_tree.node(node);

            if is_device(node.kind) {
                let table = self.networks.lan.port_table(RouterId::of(node.key)).await?;
                let Some(entry) = table.iter().find(|entry| entry.number == port) else {
                    return Ok(Err(NetUseFailure::MissingPort));
                };

                (node.key, entry.port.number)
            } else if self.networks.reach.listens(node.key, port).await? {
                (node.key, port)
            } else {
                return Ok(Err(NetUseFailure::MissingPort));
            }
        } else {
            let table = self.networks.lan.port_table(router.id).await?;
            let Some(entry) = table.iter().find(|entry| entry.number == port) else {
                return Ok(Err(NetUseFailure::MissingPort));
            };

            if is_closed(&table, port) {
                return Ok(Err(NetUseFailure::ClosedPort));
            }

            let Some(node) = lan_tree.find(&entry.lan_ip) else {
                return Ok(Err(NetUseFailure::UnknownComputer));
            };

            (lan_tree.node(node).key, entry.port.number)
        };

        self.service_session(from, target, internal, port).await
    }

    async fn service_session(
        &self,
        from: ComputerId,
        target: ComputerId,
        internal: u16,
        port: u16,
    ) -> Result<Result<Option<NetUseSession>, NetUseFailure>> {
        let Some(service) = self
            .computers
            .port(target, internal)
            .await?
            .map(|open| open.service)
        else {
            return Ok(Ok(None));
        };
        let file = self
            .files
            .lookup
            .file(target, &library_path(service.library()), true)
            .await?
            .filter(|file| !file.is_symlink());
        let Some(file) = file else {
            return Ok(Ok(None));
        };

        if file
            .data()
            .is_none_or(|data| data.kind != FileKind::Library)
        {
            return Ok(Err(NetUseFailure::InvalidLibrary(service)));
        }

        if let Some(data) = file.data()
            && !data.content.is_empty()
        {
            // `MetaExploitIntrinsics.net_use` deserializes outside a catch, unlike `connect_service`.
            serde_json::from_str::<serde_json::Value>(&data.content)
                .map_err(|error| WorldError::MalformedLibrary(error.to_string()))?;
        }

        if !self.computers.startup.is_valid_library(&file).await? {
            return Ok(Err(NetUseFailure::UnexpectedLibrary(
                file.name().to_owned(),
            )));
        }

        // `MetaExploitIntrinsics.net_use` logs the session on the computer it reaches.
        let origin = self.networks.topology.public_ip(from).await?;

        self.computers
            .logs
            .write_open_log(target, &origin, port)
            .await?;

        Ok(Ok(Some(NetUseSession::Service(Box::new(file)))))
    }
}
