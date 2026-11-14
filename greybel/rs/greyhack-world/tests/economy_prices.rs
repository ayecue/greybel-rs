use greyhack_world::{PlayerOptions, World};

use crate::support::economy_bank::get_coin;

#[tokio::test]
async fn should_set_a_coins_price_to_its_trades_average_weighted_by_coins() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let mut coin = get_coin(&world, "Bitcoin").await;

    coin.price = 90.0;
    world.modules.economy.coins.save_coin(&coin).await.unwrap();

    // 1 coin at 10 and 3 at 20: (10 + 60 + 2) / 4 = 18.
    world
        .modules
        .economy
        .market
        .record_trade(coin.id, 10, 1)
        .await
        .unwrap();
    world
        .modules
        .economy
        .market
        .record_trade(coin.id, 20, 3)
        .await
        .unwrap();

    assert_eq!(
        world
            .modules
            .economy
            .market
            .visible_price(coin.id)
            .await
            .unwrap(),
        Some(18.0)
    );

    world.modules.economy.market.update_prices().await.unwrap();

    assert_eq!(get_coin(&world, "Bitcoin").await.price, 18.0);
    assert_eq!(
        world
            .modules
            .economy
            .market
            .price_history(coin.id)
            .await
            .unwrap()
            .last()
            .unwrap()
            .price,
        18.0
    );

    // The update consumed its trades; an empty update changes nothing.
    let records = world
        .modules
        .economy
        .market
        .price_history(coin.id)
        .await
        .unwrap()
        .len();

    world.modules.economy.market.update_prices().await.unwrap();
    assert_eq!(get_coin(&world, "Bitcoin").await.price, 18.0);
    assert_eq!(
        world
            .modules
            .economy
            .market
            .price_history(coin.id)
            .await
            .unwrap()
            .len(),
        records
    );

    // Trades without a price or coins count for nothing.
    world
        .modules
        .economy
        .market
        .record_trade(coin.id, -1_000, 5)
        .await
        .unwrap();
    world
        .modules
        .economy
        .market
        .record_trade(coin.id, 30, 0)
        .await
        .unwrap();
    world.modules.economy.market.update_prices().await.unwrap();
    assert_eq!(get_coin(&world, "Bitcoin").await.price, 18.0);

    for price in 1..=10 {
        world
            .modules
            .economy
            .market
            .record_trade(coin.id, price, 1)
            .await
            .unwrap();
        world.modules.economy.market.update_prices().await.unwrap();
    }

    assert_eq!(
        world
            .modules
            .economy
            .market
            .price_history(coin.id)
            .await
            .unwrap()
            .len(),
        7
    );
}

#[tokio::test]
async fn should_show_the_order_books_middle_for_a_coin_without_a_price() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let mut coin = get_coin(&world, "Bitcoin").await;

    coin.price = 0.0;
    world.modules.economy.coins.save_coin(&coin).await.unwrap();

    assert_eq!(
        world
            .modules
            .economy
            .market
            .visible_price(coin.id)
            .await
            .unwrap(),
        Some(0.0)
    );
}
