use greyhack_behaviour::hash_password;

use crate::{Coin, CoinId, PlayerOptions, SubWallet, World};

pub(in crate::systems) async fn bank_world(source_money: f64, destination_money: f64) -> World {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();

    world.add_bank("source", "pw", source_money).await.unwrap();
    world
        .add_bank("destination", "pw", destination_money)
        .await
        .unwrap();

    world
}

pub(in crate::systems) async fn get_coin(world: &World, name: &str) -> Coin {
    world.coin_by_name(name).await.unwrap().expect("the coin")
}

pub(in crate::systems) async fn add_subwallet(
    world: &mut World,
    coin: CoinId,
    user: &str,
    balance: f64,
) {
    world
        .add_wallet(user, "pw", "0000", "source")
        .await
        .unwrap();
    world
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
