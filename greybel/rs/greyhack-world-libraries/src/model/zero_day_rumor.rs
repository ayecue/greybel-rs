use std::collections::BTreeMap;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Credentials handed out by one aligned NPC (`ZeroDaySystem.GetNpcPassword`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugCredentials {
    pub user: String,
    pub password: String,
}

/// A persisted NPC password and the player to whom its first successful login bound it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NpcDebugCredentials {
    pub password: String,
    pub player_id: Option<String>,
}

/// A debug-tools offer in the HackShop (`ZeroDaySystem.DebugToolsPlayerOffer`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugOffer {
    pub seller: String,
    pub bank: String,
    pub price: i64,
    pub user: String,
    pub password: String,
    pub purchased: bool,
}

/// The network on which one credential was first used remotely (`ZeroDayPlayerInfo.DebugToolsCreds`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugNetworkBinding {
    pub user: String,
    pub password: String,
    pub public_ip: String,
}

/// Per-player request, login, and debug-challenge state (`ZeroDayPlayerInfo`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZeroDayPlayerState {
    pub requested_at: Option<u64>,
    pub login_attempts: u32,
    pub solo_attempts: u32,
    pub solo_mail_received: bool,
    pub failed_unit_testing: bool,
    pub network_bindings: Vec<DebugNetworkBinding>,
}

/// The state saved in `InfoGen.ZeroDaySystem`, plus each player's separate `ZeroDayPlayerInfo`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZeroDayRumor {
    pub npc_passwords: BTreeMap<String, NpcDebugCredentials>,
    pub requested_credentials: BTreeMap<String, DebugCredentials>,
    /// In posting order: `ZeroDaySystem.GetBestOfferCreds` takes the first cheapest offer it iterates.
    pub offers: IndexMap<String, DebugOffer>,
    pub ip_address: String,
    pub players: BTreeMap<String, ZeroDayPlayerState>,
}

/// The result of buying one offer (`ZeroDaySystem.BuyOfferCreds`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DebugPurchase {
    Purchased(DebugOffer),
    Refused(String),
}

pub const SALE_CLOSED: &str =
    "Error: The sale cannot be made, there is no rumor available at this time.";
pub const ONE_OFFER_PER_PLAYER: &str = "Error: Only one offer can be posted per person.";
pub const INCORRECT_OFFER_CREDENTIALS: &str =
    "Error: The offer cannot be published, the credentials are incorrect.";
pub const USED_OFFER_CREDENTIALS: &str =
    "Error: The offer cannot be published, the credentials have already been used.";
pub const OFFER_NOT_AVAILABLE: &str =
    "Error: the offer no longer exists or the seller has just modified the price";
pub const CREDENTIALS_IN_USE: &str = "Error: These credentials are being used by another person.";
