use greyhack_world::{CoinId, PlayerOptions, World};

use crate::support::subwallet;

/// A world with Ether sub-wallets "alice" and "bob", whose mining cycle is an hour.
pub async fn mining_world() -> (World, CoinId) {
    let mut world = World::new(&PlayerOptions::default())
        .await
        .expect("the world generates");
    let mut ether = world
        .coin_by_name("Ether")
        .await
        .unwrap()
        .expect("Ether exists");
    let coin = ether.id;

    for user in ["alice", "bob"] {
        world
            .add_wallet(user, "pw", "pin", "test")
            .await
            .expect("the wallet is added");

        let mut account = subwallet(coin, user, 0.0);

        account.password_hash = greyhack_behaviour::hash_password("");
        world
            .add_subwallet(&account, "")
            .await
            .expect("the sub-wallet is added");
    }

    ether.cycle_hours = 1.0;
    world.save_coin(&ether).await.expect("the coin is saved");

    (world, coin)
}
