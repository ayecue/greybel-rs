use std::collections::BTreeMap;
use std::sync::LazyLock;

use serde::Deserialize;

/// Public service available to a network kind's servers (`TipoRedPublica`): service number (`ServicioID`),
/// selection probability out of 100 (100 means always), and whether the port is closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct PublicService {
    #[serde(rename = "ID")]
    pub service: u8,
    #[serde(rename = "probabilidad")]
    pub chance: u8,
    #[serde(rename = "isClosed")]
    pub closed: bool,
}

static PUBLIC: LazyLock<BTreeMap<String, Vec<PublicService>>> = LazyLock::new(|| {
    serde_json::from_str(crate::bundle::text("config/public_services.json"))
        .expect("the game's public service table is valid JSON")
});

/// Returns public services for game network kind `kind` (`ServerMap.TipoRed`) in game order, or none if the game
/// has no table for that kind.
#[must_use]
pub fn public_services(kind: &str) -> &'static [PublicService] {
    PUBLIC.get(kind).map_or(&[], Vec::as_slice)
}
