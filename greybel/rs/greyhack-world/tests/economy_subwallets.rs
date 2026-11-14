use crate::support::economy_bank::{add_subwallet, bank_world, get_coin};

#[tokio::test]
async fn should_apply_signed_subwallet_transfers_with_clamping() {
    let mut world = bank_world(0.0, 0.0).await;
    let coin = get_coin(&world, "Bitcoin").await;

    add_subwallet(&mut world, coin.id, "origin", 99_999_995.0).await;
    add_subwallet(&mut world, coin.id, "destination", 10.0).await;

    world
        .modules
        .economy
        .wallets
        .transaction(coin.id, "origin", "destination", -20, "03/Jan/2000 - 06:00")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        world
            .modules
            .economy
            .wallets
            .subwallet(coin.id, "origin")
            .await
            .unwrap()
            .unwrap()
            .balance,
        100_000_000.0
    );
    assert_eq!(
        world
            .modules
            .economy
            .wallets
            .subwallet(coin.id, "destination")
            .await
            .unwrap()
            .unwrap()
            .balance,
        0.0
    );
    assert_eq!(
        world
            .modules
            .economy
            .wallets
            .last_transaction(coin.id, "origin")
            .await
            .unwrap()
            .unwrap()
            .amount,
        -20.0
    );

    world
        .modules
        .economy
        .wallets
        .transaction(coin.id, "origin", "destination", 0, "03/Jan/2000 - 06:01")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        world
            .modules
            .economy
            .wallets
            .subwallet(coin.id, "origin")
            .await
            .unwrap()
            .unwrap()
            .balance,
        100_000_000.0
    );
    assert_eq!(
        world
            .modules
            .economy
            .wallets
            .last_transaction(coin.id, "destination")
            .await
            .unwrap()
            .unwrap()
            .amount,
        0.0
    );
}
