//! Wi-Fi networks a computer connected to, remembered by its configuration.

use crate::{NetCardFailure, Router};

/// A network remembered after connecting by Wi-Fi (`ConfigOS.SavedNetwork`). Contains what the
/// router had when the computer connected, so a changed password no longer matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedNetwork {
    pub bssid: String,
    pub essid: String,
    pub password: String,
    /// Public address of the router's network.
    pub public_ip: String,
}

impl SavedNetwork {
    /// The network `router` has now (`ConfigOS.SavedNetwork(Router)`).
    #[must_use]
    pub fn of(router: &Router) -> Self {
        Self {
            bssid: router.bssid.clone().unwrap_or_default(),
            essid: router.essid.clone().unwrap_or_default(),
            password: router.wifi_password.clone(),
            public_ip: router.public_ip.clone(),
        }
    }
}

/// Why `PlayerComputer.AutoConnectWifi` left the computer where it was
/// (`NetworkHelperServer.IwconfigUIServerRpc`). The game only tells the client's network window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavedWifiFailure {
    /// The computer has no `wlan0` card.
    NoWlan0,
    /// No router has the saved BSSID and ESSID.
    UnknownRouter,
    /// The router's network is switched off for non-payment.
    NonPayment,
    /// The router is out of range.
    OutOfRange,
    /// The router's password isn't the saved one.
    IncorrectPassword,
    /// The connection itself failed (`NetworkLan.ConnectWifi`).
    Connection(NetCardFailure),
}
