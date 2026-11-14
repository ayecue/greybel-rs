//! Network-card management and simulated Wi-Fi/Ethernet connectivity without host networking.

use greyhack_behaviour::{WIFI_RANGE, map_distance, signal_strength};

use crate::systems::NetCardService as NetCardsService;
use crate::{
    Computer, ComputerId, LanPlace, NetCard, NetCardFailure, Result, Router, RouterId,
    SavedNetwork, SavedWifiFailure,
};

impl NetCardsService {
    /// Lists in-range Wi-Fi networks for a computer, nearest first.
    ///
    /// # Errors
    ///
    /// Returns any database lookup error.
    pub async fn wifi_networks(&self, computer: ComputerId) -> Result<Vec<(String, i32, String)>> {
        let Some(source) = self.topology.router_of(computer).await? else {
            return Ok(Vec::new());
        };
        let source = source.map;
        let mut networks: Vec<_> = self
            .routers
            .routers_in_range(source)
            .await?
            .into_iter()
            .map(|router| {
                let distance = map_distance(source, router.map);
                let strength = signal_strength(distance);

                (
                    distance,
                    (
                        router.bssid.clone().unwrap_or_default(),
                        strength,
                        router.essid.clone().unwrap_or_default(),
                    ),
                )
            })
            .collect();

        networks.sort_by(|a, b| a.0.total_cmp(&b.0));

        Ok(networks.into_iter().map(|(_, network)| network).collect())
    }

    /// Returns whether a router is within Grey Hack's 0.5-unit Wi-Fi range.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn wifi_in_range(&self, computer: ComputerId, router: RouterId) -> Result<bool> {
        let Some(source) = self.topology.router_of(computer).await? else {
            return Ok(false);
        };
        let Some(target) = self.routers.router(router).await? else {
            return Ok(false);
        };

        Ok(map_distance(source.map, target.map) < WIFI_RANGE)
    }

    /// Connects a computer to main or LAN `router` Wi-Fi (`NetworkLan.ConnectWifi`). Moves it below
    /// the router at the next subnet address, or retains its address if already connected to this
    /// Wi-Fi.
    ///
    /// # Errors
    ///
    /// Returns a connection error or any database read error.
    pub async fn connect_wifi(
        &self,
        computer: ComputerId,
        router: RouterId,
    ) -> Result<Result<(), NetCardFailure>> {
        let Some(target) = self.router_rows.router(router).await? else {
            return Ok(Err(NetCardFailure::UnknownRouter));
        };
        let Some(device) = self.computer_rows.computer(computer).await? else {
            return Ok(Err(NetCardFailure::ComputerUnavailable));
        };
        let Some(lan) = self.lan.lan(target.network).await? else {
            return Ok(Err(NetCardFailure::UnknownRouter));
        };
        let on_this_wifi = device.router == Some(router)
            && self
                .active_net_card(computer)
                .await?
                .is_some_and(|card| card.wifi);
        let local_ip = if on_this_wifi {
            device.local_ip.clone()
        } else {
            lan.next_address(&target.local_ip)
        };
        let place = LanPlace::computer_under(router.computer(), router, &target.local_ip);
        let connection = self.move_to(device, &place, local_ip, true).await?;

        if connection.is_ok() {
            // `ConfigOS.SetIpAddress(Router, IP)` calls `ConfigOS.AddSavedNetwork`: the most recent last.
            let network = SavedNetwork::of(&target);
            let password_hash = self.users.register_password(&network.password).await?;

            self.computer_rows
                .add_saved_network(computer, &network, &password_hash)
                .await?;
        }

        Ok(connection)
    }

    /// Returns the Wi-Fi networks `computer` connected to, the most recent last
    /// (`ConfigOS.savedNetworks`).
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn saved_networks(&self, computer: ComputerId) -> Result<Vec<SavedNetwork>> {
        self.computer_rows.saved_networks(computer).await
    }

    /// Connects the player's computer to the most recent saved network, as it does when it starts
    /// (`PlayerComputer.AutoConnectWifi`, `NetworkHelperServer.IwconfigUIServerRpc`), or only enables
    /// Wi-Fi when none is saved. The computer stays where it is when the connection is refused.
    ///
    /// # Errors
    ///
    /// Returns any database error.
    pub async fn auto_connect_wifi(&self) -> Result<Result<(), SavedWifiFailure>> {
        let player = self.world.player();
        let Some(latest) = self.computer_rows.saved_networks(player).await?.pop() else {
            // The game marks Wi-Fi active (`IwconfigUIServerRpc` with `enable_wifi`) without checking
            // the card; without one the computer has nothing to mark.
            let _ = self.activate_card(player, true).await?;

            return Ok(Ok(()));
        };
        let cards = self.hardware_rows.net_cards(player).await?;

        if NetCard::position(&cards, "wlan0").is_none() {
            return Ok(Err(SavedWifiFailure::NoWlan0));
        }

        let Some(router) = self.saved_router(&latest).await? else {
            return Ok(Err(SavedWifiFailure::UnknownRouter));
        };

        if self.settings.is_disabled(router.id.computer()).await? {
            return Ok(Err(SavedWifiFailure::NonPayment));
        }

        if !self.wifi_in_range(player, router.id).await? {
            return Ok(Err(SavedWifiFailure::OutOfRange));
        }

        if router.wifi_password != latest.password {
            return Ok(Err(SavedWifiFailure::IncorrectPassword));
        }

        Ok(self
            .connect_wifi(player, router.id)
            .await?
            .map_err(SavedWifiFailure::Connection))
    }

    /// Finds the router of `saved`: by BSSID and ESSID, else the LAN router of that name in the
    /// network at its public address (`ServerMap.GetRouter(bssid, essid, publicIp)`).
    async fn saved_router(&self, network: &SavedNetwork) -> Result<Option<Router>> {
        if let Some(router) = self
            .routers
            .router_by_wifi(&network.bssid, &network.essid)
            .await?
        {
            return Ok(Some(router));
        }

        if network.public_ip.is_empty() {
            return Ok(None);
        }

        let Some(main) = self.routers.router_at(&network.public_ip).await? else {
            return Ok(None);
        };

        self.routers
            .lan_router_by_wifi(main.network, &network.bssid, &network.essid)
            .await
    }

    /// Connects a computer by Ethernet to `gateway` at `address` (`NetworkLan.ConnectEthernet`).
    /// Moves it below the gateway device and connects through the network main router.
    ///
    /// # Errors
    ///
    /// Returns a connection error or any database lookup error.
    pub async fn connect_ethernet(
        &self,
        computer: ComputerId,
        address: &str,
        gateway: &str,
    ) -> Result<Result<(), NetCardFailure>> {
        let Some(device) = self.computer_rows.computer(computer).await? else {
            return Ok(Err(NetCardFailure::ComputerUnavailable));
        };

        if self.settings.is_disabled(computer).await? {
            return Ok(Err(NetCardFailure::DisabledHomeNetwork));
        }

        if device.local_ip == gateway {
            return Ok(Err(NetCardFailure::SelfGateway));
        }

        let Some(lan) = self.lan.lan(device.network).await? else {
            return Ok(Err(NetCardFailure::UnknownGateway(gateway.to_owned())));
        };
        let Some(gateway_node) = lan.find(gateway) else {
            return Ok(Err(NetCardFailure::UnknownGateway(gateway.to_owned())));
        };

        if let Some(other) = lan.find(address) {
            let wifi = self
                .active_net_card(computer)
                .await?
                .is_some_and(|card| card.wifi);

            if lan.node(other).key == computer && wifi {
                return self.activate_card(computer, false).await;
            }

            return Ok(Err(NetCardFailure::AddressInUse(address.to_owned())));
        }

        let parent = if lan.node(gateway_node).is_device() {
            gateway_node
        } else {
            lan.parent(gateway_node).unwrap_or(gateway_node)
        };
        let Some(main) = self.topology.main_router(device.network).await? else {
            return Ok(Err(NetCardFailure::UnknownGateway(gateway.to_owned())));
        };
        let place = LanPlace::computer_under(lan.node(parent).key, main.id, gateway);

        self.move_to(device, &place, address.to_owned(), false)
            .await
    }

    /// Moves `device` to `place` with `local_ip` and activates its Ethernet or Wi-Fi card.
    async fn move_to(
        &self,
        mut device: Computer,
        place: &LanPlace,
        local_ip: String,
        wifi: bool,
    ) -> Result<Result<(), NetCardFailure>> {
        let computer = device.id;

        if let Err(failure) = self.activate_card(computer, wifi).await? {
            return Ok(Err(failure));
        }

        if let Some(old) = device.router.filter(|old| Some(*old) != place.router) {
            self.router_rows
                .remove_port_forwards_to(old, computer)
                .await?;
        }

        self.computer_rows.move_computer(computer, place).await?;

        device = self
            .computer_rows
            .computer(computer)
            .await?
            .unwrap_or(device);
        device.local_ip = local_ip;
        self.computer_rows.save_computer(&device).await?;

        Ok(Ok(()))
    }

    /// Activates the first Wi-Fi or Ethernet card on `computer`.
    async fn activate_card(
        &self,
        computer: ComputerId,
        wifi: bool,
    ) -> Result<Result<(), NetCardFailure>> {
        let cards = self.hardware_rows.net_cards(computer).await?;
        let Some(card) = cards.iter().position(|card| card.wifi == wifi) else {
            return Ok(Err(if wifi {
                NetCardFailure::NoWifiCard
            } else {
                NetCardFailure::NoEthernetCard
            }));
        };

        for (index, other) in cards.iter().enumerate() {
            let active = index == card;

            if other.active != active {
                self.hardware_rows
                    .save_net_card(
                        computer,
                        index,
                        &NetCard {
                            active,
                            ..other.clone()
                        },
                    )
                    .await?;
            }
        }

        Ok(Ok(()))
    }

    /// Returns network cards in installation order, which determines interface names.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn net_cards(&self, computer: ComputerId) -> Result<Vec<NetCard>> {
        self.hardware_rows.net_cards(computer).await
    }

    /// Returns the active network card on `computer`.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn active_net_card(&self, computer: ComputerId) -> Result<Option<NetCard>> {
        Ok(self
            .hardware_rows
            .net_cards(computer)
            .await?
            .into_iter()
            .find(|card| card.active))
    }

    /// Installs `card` after existing cards and returns whether the computer exists. Grey Hack
    /// manages cards through its hardware UI (`HardwareHelperServer.InstallHardware`).
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn add_net_card(&self, computer: ComputerId, card: &NetCard) -> Result<bool> {
        if !self.computer_rows.has_computer(computer).await? {
            return Ok(false);
        }

        self.hardware_rows.insert_net_card(computer, card).await?;

        Ok(true)
    }

    /// Updates active and monitor-mode flags for `computer` network card `index`.
    ///
    /// # Errors
    ///
    /// Returns any database write error.
    pub async fn save_net_card(
        &self,
        computer: ComputerId,
        index: usize,
        card: &NetCard,
    ) -> Result<()> {
        self.hardware_rows
            .save_net_card(computer, index, card)
            .await
    }
}
