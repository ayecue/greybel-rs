//! Worlds with rented networks and the player's subscriptions to them.

use greyhack_behaviour::{NetworkKind, SubscriptionKind};
use greyhack_world::{BankSubscription, NetworkId, NetworkOptions, PlayerOptions, World};

pub const FIRST: &str = "45.45.45.46";
pub const SECOND: &str = "45.45.45.47";
pub const POOR: &str = "poor";
/// More than one month, so the subscriptions made at the start are due.
pub const A_MONTH_AND_A_DAY: u64 = 32 * 86_400;

/// The player's world (its bank account is "test", with 3000) and a bank account `POOR` with 10.
pub async fn world() -> World {
    let world = World::new(&PlayerOptions::default()).await.unwrap();

    world
        .modules
        .economy
        .bank
        .add_bank(POOR, "pw", 10.0)
        .await
        .unwrap();

    world
}

/// Makes a rented network at `public_ip` and returns it.
pub async fn rented(world: &mut World, public_ip: &str) -> NetworkId {
    let router = world
        .modules
        .creation
        .network_generation
        .add_network(&NetworkOptions {
            public_ip: Some(public_ip.to_owned()),
            ..NetworkOptions::default()
        })
        .await
        .unwrap();
    let network = world
        .modules
        .core
        .networks
        .routers
        .router(router)
        .await
        .unwrap()
        .unwrap()
        .network;

    world
        .modules
        .core
        .networks
        .settings
        .set_network_kind(network, NetworkKind::RentServer)
        .await
        .unwrap();

    network
}

/// Subscribes `bank` to the rental of the network at `public_ip` for what the network costs, as renting it does
/// (`BankHelperServer.RentServer`).
pub async fn rent(world: &mut World, bank: &str, network: NetworkId, public_ip: &str) -> i64 {
    let price = world
        .modules
        .economy
        .bank
        .rental_price(network)
        .await
        .unwrap();

    assert!(
        world
            .modules
            .economy
            .bank
            .add_subscription(bank, SubscriptionKind::Rented, price, public_ip)
            .await
            .unwrap()
    );

    price
}

/// Whether the network is disabled for nonpayment.
pub async fn disabled(world: &World, network: NetworkId) -> bool {
    world
        .modules
        .core
        .networks
        .settings
        .network(network)
        .await
        .unwrap()
        .unwrap()
        .disabled
}

/// The money of bank account `bank`.
pub async fn money(world: &World, bank: &str) -> f64 {
    world
        .modules
        .economy
        .bank
        .bank_account(bank)
        .await
        .unwrap()
        .unwrap()
        .money
}

/// Monthly subscription `kind` of bank account `bank`.
pub async fn subscription(
    world: &World,
    bank: &str,
    kind: SubscriptionKind,
) -> Option<BankSubscription> {
    world
        .modules
        .economy
        .bank
        .bank_subscription(bank, kind)
        .await
        .unwrap()
}

/// The payment routines among `routines`.
pub fn payments_of(routines: &[&'static str]) -> Vec<&'static str> {
    routines
        .iter()
        .copied()
        .filter(|key| *key == "economy.payment")
        .collect()
}

/// The dates the world's payments wait for, earliest first.
pub async fn payment_dates(world: &World) -> Vec<u64> {
    world
        .modules
        .routines
        .scheduled_routines()
        .await
        .unwrap()
        .into_iter()
        .filter(|entry| entry.key == "economy.payment")
        .map(|entry| entry.at)
        .collect()
}
