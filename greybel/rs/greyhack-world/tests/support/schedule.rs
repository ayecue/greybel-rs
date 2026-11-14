use greyhack_world::{CoinId, PlayerOptions, ScheduledRoutine, World};

use crate::support::subwallet;

/// A world with Ether sub-wallets "alice" and "bob", whose mining cycle is an hour.
pub async fn mining_world() -> (World, CoinId) {
    let world = World::new(&PlayerOptions::default())
        .await
        .expect("the world generates");
    let mut ether = world
        .modules
        .economy
        .coins
        .coin_by_name("Ether")
        .await
        .unwrap()
        .expect("Ether exists");
    let coin = ether.id;

    for user in ["alice", "bob"] {
        world
            .modules
            .economy
            .wallets
            .add_wallet(user, "pw", "pin", "test")
            .await
            .expect("the wallet is added");

        let mut account = subwallet(coin, user, 0.0);

        account.password_hash = greyhack_behaviour::hash_password("");
        world
            .modules
            .economy
            .wallets
            .add_subwallet(&account, "")
            .await
            .expect("the sub-wallet is added");
    }

    ether.cycle_hours = 1.0;
    world
        .modules
        .economy
        .coins
        .save_coin(&ether)
        .await
        .expect("the coin is saved");

    (world, coin)
}

/// The scheduled routines but the periodic check for library versions, which every world has.
pub async fn scheduled_work(world: &World) -> Vec<ScheduledRoutine> {
    world
        .modules
        .routines
        .scheduled_routines()
        .await
        .expect("the routines are read")
        .into_iter()
        .filter(|entry| !["libraries.version_check", "economy.daily_upkeep"].contains(&entry.key))
        .collect()
}
