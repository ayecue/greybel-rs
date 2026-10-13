//! Network cards, Wi-Fi, and Ethernet of a computer as scripts see them (`ComputerIntrinsics`).

use greyhack_behaviour::{GUEST_USER, NetworkKind, is_valid_ip, same_subnet};

use crate::systems::computer::failure::ComputerFailure;
use crate::{ComputerId, NetCard, Result, World};

/// Network card of a computer, as `network_devices` lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkDevice {
    /// Interface name, `ethN` or `wlanN`.
    pub name: String,
    /// First word of the card description.
    pub model: String,
    /// Whether monitor mode is on.
    pub monitor: bool,
}

/// Kind of the active network card (`ComputerIntrinsics.active_net_card`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveCard {
    /// No card is active.
    None,
    /// A Wi-Fi card is active.
    Wifi,
    /// An Ethernet card is active.
    Ethernet,
}

/// Result of listing Wi-Fi networks (`ComputerIntrinsics.wifi_networks`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WifiScan {
    /// The computer has no card of the name.
    NoDevice,
    /// The card is not a Wi-Fi card.
    NotWifi,
    /// No card is active.
    Inactive,
    /// BSSID, power in percent, and ESSID of each network in reach.
    Networks(Vec<(String, i32, String)>),
}

/// A script's request to join a Wi-Fi network; each field is absent when the script gave no text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WifiRequest<'a> {
    /// Interface name.
    pub device: Option<&'a str>,
    /// BSSID of the router.
    pub bssid: Option<&'a str>,
    /// ESSID of the router.
    pub essid: Option<&'a str>,
    /// Wi-Fi password.
    pub password: Option<&'a str>,
}

/// What a network switch did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitchResult {
    /// The computer is gone, or the script gave no text for an argument.
    Null,
    /// The computer joined the network.
    Connect,
    /// The request is refusal.
    Refusal(ComputerFailure),
}

/// A network switch and whether it starts the wait before the next switch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkSwitch {
    /// What the switch did.
    pub result: SwitchResult,
    /// Whether a new switch must wait.
    pub starts_wait: bool,
}

impl NetworkSwitch {
    fn refusal(failure: ComputerFailure, starts_wait: bool) -> Self {
        Self {
            result: SwitchResult::Refusal(failure),
            starts_wait,
        }
    }
}

impl World {
    /// Lists the network cards of `computer` (`ComputerIntrinsics.network_devices`), or `None` if it
    /// is gone.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn network_devices(
        &self,
        computer: ComputerId,
    ) -> Result<Option<Vec<NetworkDevice>>> {
        if self.computer(computer).await?.is_none() {
            return Ok(None);
        }

        let cards = self.net_cards(computer).await?;
        let names = NetCard::interface_names(&cards);

        Ok(Some(
            cards
                .into_iter()
                .zip(names)
                .map(|(card, name)| NetworkDevice {
                    name,
                    model: card.part.name.split(' ').next().unwrap_or("").to_owned(),
                    monitor: card.monitor_enabled,
                })
                .collect(),
        ))
    }

    /// Returns the kind of the active network card (`ComputerIntrinsics.active_net_card`), or `None`
    /// if `computer` is gone.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn active_card_kind(&self, computer: ComputerId) -> Result<Option<ActiveCard>> {
        if self.computer(computer).await?.is_none() {
            return Ok(None);
        }

        Ok(Some(match self.active_net_card(computer).await? {
            None => ActiveCard::None,
            Some(card) if card.wifi => ActiveCard::Wifi,
            Some(_) => ActiveCard::Ethernet,
        }))
    }

    /// Lists the Wi-Fi networks `device` of `computer` reaches (`ComputerIntrinsics.wifi_networks`),
    /// or `None` if the computer is gone.
    ///
    /// # Errors
    ///
    /// Returns any database read error.
    pub async fn scan_wifi(&self, computer: ComputerId, device: &str) -> Result<Option<WifiScan>> {
        if self.computer(computer).await?.is_none() {
            return Ok(None);
        }

        let cards = self.net_cards(computer).await?;
        let Some(index) = NetCard::position(&cards, device) else {
            return Ok(Some(WifiScan::NoDevice));
        };

        if !cards[index].wifi {
            return Ok(Some(WifiScan::NotWifi));
        }

        if self.active_net_card(computer).await?.is_none() {
            return Ok(Some(WifiScan::Inactive));
        }

        Ok(Some(WifiScan::Networks(
            self.wifi_networks(computer).await?,
        )))
    }

    /// Joins a Wi-Fi network as `user` (`ComputerIntrinsics.connect_wifi`). Guest users are
    /// rejected, missing arguments give null, and `wait` reports a reconnect attempted too soon.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn join_wifi(
        &mut self,
        computer: ComputerId,
        user: &str,
        request: &WifiRequest<'_>,
        wait: bool,
    ) -> Result<NetworkSwitch> {
        let null = NetworkSwitch {
            result: SwitchResult::Null,
            starts_wait: false,
        };

        if user == GUEST_USER {
            return Ok(NetworkSwitch::refusal(ComputerFailure::GuestUser, false));
        }

        let WifiRequest {
            device: Some(device),
            bssid: Some(bssid),
            essid: Some(essid),
            password: Some(password),
        } = *request
        else {
            return Ok(null);
        };

        let Some(row) = self.computer(computer).await? else {
            return Ok(null);
        };
        let cards = self.net_cards(computer).await?;
        let Some(index) = NetCard::position(&cards, device) else {
            return Ok(NetworkSwitch::refusal(
                ComputerFailure::MissingNetDevice,
                false,
            ));
        };

        if !cards[index].wifi {
            return Ok(NetworkSwitch::refusal(
                ComputerFailure::WifiCardsOnly,
                false,
            ));
        }

        if wait {
            return Ok(NetworkSwitch::refusal(
                ComputerFailure::ReconnectTooSoon,
                false,
            ));
        }

        // Prefer a reachable router on the computer's current network (`NetworkLan.FindRouterLan`).
        let router = match self.lan_router_by_wifi(row.network, bssid, essid).await? {
            Some(router) => router,
            None => {
                let Some(router) = self.router_by_wifi(bssid, essid).await? else {
                    return Ok(NetworkSwitch::refusal(
                        ComputerFailure::UnknownWifi {
                            bssid: bssid.to_owned(),
                            essid: essid.to_owned(),
                        },
                        true,
                    ));
                };

                // Rented-server computers cannot join other networks (`ConfigOS.isRented`).
                if row.network_kind == NetworkKind::RentServer
                    && self.public_ip(computer).await? != router.public_ip
                {
                    return Ok(NetworkSwitch::refusal(
                        ComputerFailure::RentedComputer,
                        true,
                    ));
                }

                if self.is_disabled(router.id.computer()).await? {
                    return Ok(NetworkSwitch::refusal(
                        ComputerFailure::DisabledNetwork,
                        true,
                    ));
                }

                if !self.wifi_in_range(computer, router.id).await? {
                    return Ok(NetworkSwitch::refusal(ComputerFailure::OutOfReach, true));
                }

                router
            }
        };

        if router.wifi_password != password {
            return Ok(NetworkSwitch::refusal(
                ComputerFailure::IncorrectWifiPassword,
                true,
            ));
        }

        Ok(NetworkSwitch {
            result: match self.connect_wifi(computer, router.id).await? {
                Ok(()) => SwitchResult::Connect,
                Err(failure) => SwitchResult::Refusal(ComputerFailure::NetCard(failure)),
            },
            starts_wait: true,
        })
    }

    /// Applies Ethernet settings as `user` (`ComputerIntrinsics.connect_ethernet`); `wait` reports a
    /// network switch attempted less than a second after the previous one.
    ///
    /// # Errors
    ///
    /// Returns any database read or write error.
    pub async fn join_ethernet(
        &mut self,
        computer: ComputerId,
        user: &str,
        (device, address, gateway): (&str, &str, &str),
        wait: bool,
    ) -> Result<NetworkSwitch> {
        if self.computer(computer).await?.is_none() {
            return Ok(NetworkSwitch {
                result: SwitchResult::Null,
                starts_wait: false,
            });
        }

        if user == GUEST_USER {
            return Ok(NetworkSwitch::refusal(ComputerFailure::GuestUser, false));
        }

        if !is_valid_ip(address) {
            return Ok(NetworkSwitch::refusal(
                ComputerFailure::InvalidAddress,
                false,
            ));
        }

        if !is_valid_ip(gateway) {
            return Ok(NetworkSwitch::refusal(
                ComputerFailure::InvalidGateway,
                false,
            ));
        }

        if !same_subnet(address, gateway) {
            return Ok(NetworkSwitch::refusal(
                ComputerFailure::DifferentSubnets,
                false,
            ));
        }

        let cards = self.net_cards(computer).await?;
        let Some(index) = NetCard::position(&cards, device) else {
            return Ok(NetworkSwitch::refusal(
                ComputerFailure::MissingNetDevice,
                false,
            ));
        };

        if cards[index].wifi {
            return Ok(NetworkSwitch::refusal(
                ComputerFailure::EthernetCardsOnly,
                false,
            ));
        }

        if wait {
            return Ok(NetworkSwitch::refusal(
                ComputerFailure::ReconnectTooSoon,
                false,
            ));
        }

        Ok(NetworkSwitch {
            result: match self.connect_ethernet(computer, address, gateway).await? {
                Ok(()) => SwitchResult::Connect,
                Err(failure) => SwitchResult::Refusal(ComputerFailure::NetCard(failure)),
            },
            starts_wait: true,
        })
    }
}
