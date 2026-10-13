use greyhack_behaviour::hash_password;

use crate::{Offer, PlayerOptions, Side, SubWallet, WalletId, World};

/// A world whose wallet `seller` has 40 Bitcoin and whose wallet `buyer` has $100000 in its bank.
pub(super) async fn market() -> (World, WalletId, WalletId) {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();

    world.add_bank("seller-bank", "pw", 0.0).await.unwrap();
    world.add_bank("buyer-bank", "pw", 100_000.0).await.unwrap();

    let seller = world
        .add_wallet("seller", "pw", "0000", "seller-bank")
        .await
        .unwrap();
    let buyer = world
        .add_wallet("buyer", "pw", "0000", "buyer-bank")
        .await
        .unwrap();

    let coin = world.coin_id("Bitcoin").await.unwrap().unwrap();

    for (wallet, user, amount) in [(seller, "seller-sub", 40.0), (buyer, "buyer-sub", 0.0)] {
        world
            .add_subwallet(
                &SubWallet {
                    coin,
                    user: user.to_owned(),
                    password_hash: hash_password("pw"),
                    balance: amount,
                    info: String::new(),
                    wallet_user: world.wallet(wallet).await.unwrap().unwrap().user,
                },
                "pw",
            )
            .await
            .unwrap();
        world.add_balance(wallet, "Bitcoin", amount).await.unwrap();
    }

    (world, seller, buyer)
}

/// The balance of `wallet` in `coin`.
pub(super) async fn balance(world: &World, wallet: WalletId, coin: &str) -> Option<f64> {
    world.balance(wallet, coin).await.unwrap()
}

/// The money of bank account `number`.
pub(super) async fn money(world: &World, number: &str) -> f64 {
    world
        .bank_account(number)
        .await
        .unwrap()
        .expect("the bank account")
        .money
}

/// The offers on `coin` named `name` on `side`.
pub(super) async fn offers(world: &World, name: &str, side: Side) -> Vec<Offer> {
    let coin = world.coin_by_name(name).await.unwrap().expect("the coin");

    world.offers(coin.id, side).await.unwrap()
}
