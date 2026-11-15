use greyhack_behaviour::hash_password;
use greyhack_world::{PlayerOptions, World};
use greyhack_world_core::CoinId;
use greyhack_world_economy::{Coin, SubWallet};

pub async fn bank_world(source_money: f64, destination_money: f64) -> World {
    let world = World::new(&PlayerOptions::default()).await.unwrap();

    world
        .modules
        .economy
        .bank
        .add_bank("source", "pw", source_money)
        .await
        .unwrap();
    world
        .modules
        .economy
        .bank
        .add_bank("destination", "pw", destination_money)
        .await
        .unwrap();

    world
}

pub async fn get_coin(world: &World, name: &str) -> Coin {
    world
        .modules
        .economy
        .coins
        .coin_by_name(name)
        .await
        .unwrap()
        .expect("the coin")
}

pub async fn add_subwallet(world: &mut World, coin: CoinId, user: &str, balance: f64) {
    world
        .modules
        .economy
        .wallets
        .add_wallet(user, "pw", "0000", "source")
        .await
        .unwrap();
    world
        .modules
        .economy
        .wallets
        .add_subwallet(
            &SubWallet {
                coin,
                user: user.to_owned(),
                password_hash: hash_password("pw"),
                balance,
                info: String::new(),
                wallet_user: user.to_owned(),
            },
            "pw",
        )
        .await
        .unwrap();
}
