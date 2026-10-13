use greyhack_behaviour::SubscriptionKind;

use crate::systems::economy::bank::tests::support::bank_world;

#[tokio::test]
async fn should_apply_only_due_subscriptions_and_record_insufficient_funds() {
    let mut world = bank_world(100.0, 20.0).await;

    assert!(
        world
            .add_subscription("source", SubscriptionKind::Bank, 50, "")
            .await
            .unwrap()
    );
    assert!(
        world
            .add_subscription("destination", SubscriptionKind::Rented, 50, "")
            .await
            .unwrap()
    );

    world.run_subscription_payments().await.unwrap();
    assert!(
        world
            .bank_account("source")
            .await
            .unwrap()
            .unwrap()
            .transactions
            .is_empty()
    );

    world.set_date("03/Jan/2001 - 06:00");
    world.run_subscription_payments().await.unwrap();

    let paid = world.bank_account("source").await.unwrap().unwrap();
    let failed = world.bank_account("destination").await.unwrap().unwrap();

    assert_eq!(paid.money, 50.0);
    assert_eq!(paid.transactions.len(), 1);
    assert_eq!(paid.transactions[0].amount, -50);
    assert_eq!(paid.transactions[0].reason, "Monthly Payment");
    assert!(paid.transactions[0].success);
    assert_eq!(failed.money, 20.0);
    assert_eq!(failed.transactions.len(), 1);
    assert_eq!(failed.transactions[0].amount, -50);
    assert!(!failed.transactions[0].success);

    // Each due run advances one calendar month, so running again at the same time does not double-charge.
    world.run_subscription_payments().await.unwrap();
    assert_eq!(
        world.bank_account("source").await.unwrap().unwrap().money,
        50.0
    );
    assert_eq!(
        world
            .bank_account("source")
            .await
            .unwrap()
            .unwrap()
            .transactions
            .len(),
        1
    );

    world
        .add_subscription("source", SubscriptionKind::Bank, 25, "")
        .await
        .unwrap();
    assert_eq!(
        world
            .bank_subscription("source", SubscriptionKind::Bank)
            .await
            .unwrap()
            .unwrap()
            .amount,
        75
    );
    world
        .add_subscription("source", SubscriptionKind::Bank, -75, "")
        .await
        .unwrap();
    assert!(
        world
            .bank_subscription("source", SubscriptionKind::Bank)
            .await
            .unwrap()
            .is_none()
    );
}
