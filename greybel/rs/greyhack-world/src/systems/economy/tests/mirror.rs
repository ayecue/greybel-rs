use greyhack_behaviour::SubscriptionKind;

use crate::systems::economy::bank::tests::support::{bank_world, get_coin};
use crate::{Database, World};

#[tokio::test]
async fn should_keep_new_economy_state_when_saving_and_reloading() {
    let mut world = bank_world(100.0, 0.0).await;

    world
        .bank_transfer("source", "pw", "destination", 100)
        .await
        .unwrap()
        .unwrap();
    world
        .add_subscription("source", SubscriptionKind::Admin, 50, "")
        .await
        .unwrap();

    let coin = get_coin(&world, "Bitcoin").await;

    world.record_trade(coin.id, 10, 1).await.unwrap();

    let database = Database::in_memory().await.unwrap();

    world.save_into(&database).await.unwrap();

    let mut loaded = World::load(&database).await.unwrap();

    assert_eq!(
        loaded
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
            .bank_subscription("source", SubscriptionKind::Admin)
            .await
            .unwrap()
            .is_some()
    );
    loaded.update_prices().await.unwrap();
    assert_eq!(
        loaded
            .price_history(coin.id)
            .await
            .unwrap()
            .last()
            .unwrap()
            .price,
        10.0
    );
}
