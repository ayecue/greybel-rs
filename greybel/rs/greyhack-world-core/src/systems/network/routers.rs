//! Router persistence, lookup by address/domain/Wi-Fi, and kernel access.

use std::rc::Rc;

use greyhack_behaviour::{
    DeviceKind, FileKind, KERNEL_ROUTER_PATH, ServiceId, WIFI_RANGE, is_lan_ip, library_path,
    map_distance,
};
use greyhack_world_modules::Dep;

use crate::storage::{ComputerRepository, RouterRepository};
use crate::systems::computer::hardware::Hardware;
use crate::systems::{
    FileLookup, LibraryFileService, NetworkLan, NetworkTopology, RouterService as RoutersService,
    Startup, UserService,
};
use crate::{
    ComputerId, LanPlace, NetworkId, Node, Result, Router, RouterDraft, RouterId, TablePort,
};

/// Ports `device_ports` lists for an address (`RouterIntrinsics.device_ports`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PortLookup {
    /// The router, its network, the caller's computer or the device does not exist, or the address is
    /// not one a remote caller may query.
    Missing,
    /// The device does not respond to the caller.
    Unreachable,
    /// The ports the caller can see.
    Ports(Vec<TablePort>),
}

impl RoutersService {
    /// Inserts a router draft, composing its computer, password, and router rows in write order.
    pub async fn insert_router_with<L, S>(
        &self,
        draft: &RouterDraft<L, S>,
        network: NetworkId,
        place: Option<&LanPlace>,
        library_id: impl AsyncFn(&L) -> Result<crate::LibraryId>,
    ) -> Result<RouterId> {
        let id = RouterId(
            self.hardware
                .insert_computer_at_with(&draft.computer, network, place, library_id)
                .await?
                .0,
        );
        let password_hash = self.users.register_password(&draft.wifi_password).await?;

        self.router_rows
            .insert_router_row(draft, id, &password_hash)
            .await
    }

    /// Returns router `id`, or `None` if removed.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn router(&self, id: RouterId) -> Result<Option<Router>> {
        self.router_rows.router(id).await
    }

    /// Updates the `router` row.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn save_router(&self, router: &Router) -> Result<()> {
        let password_hash = self.users.register_password(&router.wifi_password).await?;

        self.router_rows.save_router(router, &password_hash).await
    }

    /// Returns the router at `public_ip`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn router_at(&self, public_ip: &str) -> Result<Option<Router>> {
        self.router_rows.router_at(public_ip).await
    }

    /// Returns the public address for `domain`, case-insensitively.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn lookup(&self, domain: &str) -> Result<Option<String>> {
        Ok(self
            .router_rows
            .router_of_domain(domain)
            .await?
            .map(|router| router.public_ip))
    }

    /// Returns the router with case-insensitive `bssid` and matching `essid`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn router_by_wifi(&self, bssid: &str, essid: &str) -> Result<Option<Router>> {
        self.router_rows.router_of_wifi(bssid, essid).await
    }

    /// Returns network routers and switches with the main router first.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn routers_in(&self, network: NetworkId) -> Result<Vec<Router>> {
        self.router_rows.routers_of(network).await
    }

    /// Returns the first main or LAN router in `network` with `bssid` and `essid`, using
    /// `NetworkLan.FindRouterLan` search order.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn lan_router_by_wifi(
        &self,
        network: NetworkId,
        bssid: &str,
        essid: &str,
    ) -> Result<Option<Router>> {
        let Some(lan) = self.lan.lan(network).await? else {
            return Ok(None);
        };
        let routers = self.router_rows.routers_of(network).await?;

        Ok(lan
            .search_order()
            .into_iter()
            .filter_map(|node| {
                routers
                    .iter()
                    .find(|router| router.id.computer() == lan.node(node).key)
            })
            .find(|router| {
                router.kind == DeviceKind::Router
                    && router.bssid.as_deref() == Some(bssid)
                    && router.essid.as_deref() == Some(essid)
            })
            .cloned())
    }

    /// Returns main routers with Wi-Fi in range of `map`, ordered by ID. `wifi_networks` excludes
    /// LAN routers.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn routers_in_range(&self, map: [f32; 2]) -> Result<Vec<Router>> {
        // Expand bounds slightly beyond Wi-Fi range so rounding cannot exclude an in-range router.
        let mut routers = self
            .router_rows
            .routers_near(map, WIFI_RANGE + 0.001)
            .await?;

        routers.retain(|router| map_distance(map, router.map) < WIFI_RANGE);

        Ok(routers)
    }

    /// Returns every network public address, ordered by ID.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn public_addresses(&self) -> Result<Vec<String>> {
        self.router_rows.public_addresses().await
    }

    /// Returns the computer at `lan_ip` behind `public_ip`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn computer_at(&self, public_ip: &str, lan_ip: &str) -> Result<Option<ComputerId>> {
        let Some(router) = self.router_rows.router_at(public_ip).await? else {
            return Ok(None);
        };

        self.topology.device(router.network, lan_ip).await
    }

    /// Returns the direct, non-symlink `/lib/kernel_router.so` file for `router`
    /// (`MetaExploitIntrinsics.net_use`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn kernel_file(&self, router: RouterId) -> Result<Option<Node>> {
        Ok(self
            .lookup
            .file(router.computer(), KERNEL_ROUTER_PATH, false)
            .await?
            .filter(|file| !file.is_symlink()))
    }

    /// Returns the library in `router`'s [`Self::kernel_file`].
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn kernel_library(&self, router: RouterId) -> Result<Option<crate::Library>> {
        match self.kernel_file(router).await? {
            Some(file) => self.library_files.library_of(&file).await,
            None => Ok(None),
        }
    }

    /// Returns the service and library version at `lan_ip` `internal_port` as resolved by `router`
    /// (`Router.GetPortVersion`, `Router.GetComputer`). Requires a matching port and a same-named
    /// service library in `/lib` after symlink resolution. Returns `None` otherwise.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn port_service(
        &self,
        router: RouterId,
        lan_ip: &str,
        internal_port: u16,
    ) -> Result<Option<(ServiceId, String)>> {
        let Some(router) = self.router_rows.router(router).await? else {
            return Ok(None);
        };
        let computer = if lan_ip == router.local_ip {
            Some(router.id.computer())
        } else {
            self.topology.device(router.network, lan_ip).await?
        };
        let Some(computer) = computer else {
            return Ok(None);
        };
        let Some(port) = self
            .computer_rows
            .ports(computer)
            .await?
            .into_iter()
            .find(|port| port.number == internal_port)
        else {
            return Ok(None);
        };
        let path = library_path(port.service.library());
        let Some(file) = self.lookup.file(computer, &path, true).await? else {
            return Ok(None);
        };

        if !self.startup.is_valid_library(&file).await? {
            return Ok(None);
        }

        Ok(self
            .library_files
            .library_of(&file)
            .await?
            .map(|library| (port.service, library.version)))
    }

    /// Returns the resolved `kernel_router.so` version for `router` when it is a binary library
    /// (`RouterIntrinsics.kernel_version`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn kernel_version(&self, router: RouterId) -> Result<Option<String>> {
        let Some(file) = self
            .lookup
            .file(router.computer(), KERNEL_ROUTER_PATH, true)
            .await?
        else {
            return Ok(None);
        };

        if !file
            .data()
            .is_some_and(|data| data.kind == FileKind::Library && data.is_binary())
        {
            return Ok(None);
        }

        Ok(self
            .library_files
            .library_of(&file)
            .await?
            .map(|library| library.version))
    }

    /// Returns the Wi-Fi ESSID of `router`, empty for a router without Wi-Fi
    /// (`RouterIntrinsics.essid_name`), or `None` if removed.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn essid_name(&self, router: RouterId) -> Result<Option<String>> {
        Ok(self
            .router(router)
            .await?
            .map(|router| router.essid.unwrap_or_default()))
    }

    /// Returns the Wi-Fi BSSID of `router`, empty for a router without Wi-Fi
    /// (`RouterIntrinsics.bssid_name`), or `None` if removed.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn bssid_name(&self, router: RouterId) -> Result<Option<String>> {
        Ok(self
            .router(router)
            .await?
            .map(|router| router.bssid.unwrap_or_default()))
    }

    /// Lists the visible local ports and forwards of `router` (`RouterIntrinsics.used_ports`), or
    /// `None` if removed.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn visible_table_ports(&self, router: RouterId) -> Result<Option<Vec<TablePort>>> {
        if self.router(router).await?.is_none() {
            return Ok(None);
        }

        Ok(Some(
            self.lan
                .port_table(router)
                .await?
                .into_iter()
                .filter(|port| port.port.visible)
                .collect(),
        ))
    }

    /// Looks up `number` in the port table of `router` (`Puertos.GetPuertoInfoRouter`), or returns
    /// `None` if the router is removed or has no such port.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn table_port(&self, router: RouterId, number: i32) -> Result<Option<TablePort>> {
        if self.router(router).await?.is_none() {
            return Ok(None);
        }

        let Ok(number) = u16::try_from(number) else {
            return Ok(None);
        };

        Ok(self
            .lan
            .port_table(router)
            .await?
            .into_iter()
            .find(|port| port.number == number))
    }

    /// Lists ports for `address` on the network of `router` for a script on `computer`
    /// (`RouterIntrinsics.device_ports`). Remote callers can access only addresses listed by the main
    /// router. The target must respond to the caller's computer for local calls, or to the router for
    /// remote calls.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn device_ports(
        &self,
        router: RouterId,
        address: &str,
        computer: ComputerId,
    ) -> Result<PortLookup> {
        let Some(router_row) = self.router(router).await? else {
            return Ok(PortLookup::Missing);
        };
        let Some(lan) = self.lan.lan(router_row.network).await? else {
            return Ok(PortLookup::Missing);
        };
        let Some(host) = self.topology.computer(computer).await? else {
            return Ok(PortLookup::Missing);
        };
        let remote = host.network != router_row.network;
        let main_ip = &lan.node(lan.root()).local_ip;

        if remote
            && !lan
                .router_lan(main_ip, false)
                .iter()
                .any(|lan_ip| lan_ip == address)
        {
            return Ok(PortLookup::Missing);
        }

        let from = if remote {
            &router_row.local_ip
        } else {
            &host.local_ip
        };

        if !lan.ping(from, address) {
            return Ok(PortLookup::Unreachable);
        }

        let Some(device) = lan.find(address) else {
            return Ok(PortLookup::Missing);
        };
        let node = lan.node(device);
        let listable = |port: &crate::Port| is_lan_ip(address) || port.visible;
        let ports = if matches!(node.kind, DeviceKind::Router | DeviceKind::Switch) {
            self.lan
                .port_table(RouterId::of(node.key))
                .await?
                .into_iter()
                .filter(|port| listable(&port.port))
                .collect()
        } else {
            self.computer_rows
                .ports(node.key)
                .await?
                .into_iter()
                .filter(listable)
                .map(|port| TablePort {
                    number: port.number,
                    lan_ip: address.to_owned(),
                    port,
                })
                .collect()
        };

        Ok(PortLookup::Ports(ports))
    }
}

/// Router lookup, ports, and forwarded connections.
pub struct RouterService {
    pub(crate) computer_rows: Rc<ComputerRepository>,
    pub(crate) router_rows: Rc<RouterRepository>,
    pub(crate) hardware: Dep<Hardware>,
    pub(crate) users: Dep<UserService>,
    pub(crate) startup: Dep<Startup>,
    pub(crate) lookup: Dep<FileLookup>,
    pub(crate) library_files: Dep<LibraryFileService>,
    pub(crate) lan: Dep<NetworkLan>,
    pub(crate) topology: Dep<NetworkTopology>,
}

impl RouterService {
    pub(crate) fn new(
        registry: &Rc<greyhack_world_modules::Registry>,
        computer_rows: Rc<ComputerRepository>,
        router_rows: Rc<RouterRepository>,
    ) -> Self {
        Self {
            computer_rows,
            router_rows,
            hardware: registry.get(),
            users: registry.get(),
            startup: registry.get(),
            lookup: registry.get(),
            library_files: registry.get(),
            lan: registry.get(),
            topology: registry.get(),
        }
    }
}
