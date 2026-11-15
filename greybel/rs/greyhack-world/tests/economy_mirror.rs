use greyhack_behaviour::SubscriptionKind;
use greyhack_world::{DatabaseOpen, World};
use greyhack_world_core::Database;

use crate::support::economy_bank::{bank_world, get_coin};

#[tokio::test]
async fn should_keep_new_economy_state_when_saving_and_reloading() {
    let world = bank_world(100.0, 0.0).await;

    world
        .modules
        .economy
        .bank
        .bank_transfer("source", "pw", "destination", 100)
        .await
        .unwrap()
        .unwrap();
    world
        .modules
        .economy
        .bank
        .add_subscription("source", SubscriptionKind::Admin, 50, "")
        .await
        .unwrap();

    let coin = get_coin(&world, "Bitcoin").await;

    world
        .modules
        .economy
        .market
        .record_trade(coin.id, 10, 1)
        .await
        .unwrap();

    let database = Database::in_memory().await.unwrap();

    world.save_into(&database).await.unwrap();

    let loaded = World::load(&database).await.unwrap();

    assert_eq!(
        loaded
            .modules
            .economy
            .bank
            .bank_account("source")
            .await
            .unwrap()
            .unwrap()
            .transactions
            .len(),
        2
    );
    assert_eq!(
        loaded
            .modules
            .economy
            .bank
            .bank_account("destination")
            .await
            .unwrap()
            .unwrap()
            .transactions
            .len(),
        1
    );
    assert!(
        loaded
            .modules
            .economy
            .bank
            .bank_subscription("source", SubscriptionKind::Admin)
            .await
            .unwrap()
            .is_some()
    );
    loaded.modules.economy.market.update_prices().await.unwrap();
    assert_eq!(
        loaded
            .modules
            .economy
            .market
            .price_history(coin.id)
            .await
            .unwrap()
            .last()
            .unwrap()
            .price,
        10.0
    );
}
