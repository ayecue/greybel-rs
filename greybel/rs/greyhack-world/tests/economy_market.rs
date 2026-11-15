use greyhack_behaviour::Side;
use greyhack_world_economy::Trade;

use crate::support::economy_market::{balance, market, money, offers};

#[tokio::test]
async fn should_wait_for_buyers_when_selling_into_an_empty_market() {
    // Arrange
    let (world, seller, _) = market().await;

    // Act
    let message = world
        .modules
        .economy
        .market
        .sell_coin("Bitcoin", 40.0, 10.0, seller, "seller-sub")
        .await
        .unwrap();

    // Assert
    assert_eq!(message, Some(Ok(Trade::Pending)));
}

#[tokio::test]
async fn should_trade_what_is_offered_and_wait_for_the_rest_of_a_larger_buy() {
    // Arrange
    let (world, seller, buyer) = market().await;
    let _ = world
        .modules
        .economy
        .market
        .sell_coin("Bitcoin", 40.0, 10.0, seller, "seller-sub")
        .await
        .unwrap();

    // Act
    let message = world
        .modules
        .economy
        .market
        .buy_coin("Bitcoin", 60.0, 10.0, buyer, "buyer-sub")
        .await
        .unwrap();

    // Assert
    assert_eq!(message, Some(Ok(Trade::Partial)));
    assert_eq!(balance(&world, buyer, "Bitcoin").await, Some(40.0));
    assert_eq!(balance(&world, seller, "Bitcoin").await, Some(0.0));
    assert_eq!(money(&world, "seller-bank").await, 400.0);
    assert!(offers(&world, "Bitcoin", Side::Sell).await.is_empty());

    let buy_offers = offers(&world, "Bitcoin", Side::Buy).await;

    assert_eq!(buy_offers.len(), 1);
    assert_eq!(buy_offers[0].amount, 20.0);
}

#[tokio::test]
async fn should_wait_for_sellers_when_buying_from_an_empty_market() {
    // Arrange
    let (world, _, buyer) = market().await;

    // Act
    let message = world
        .modules
        .economy
        .market
        .buy_coin("Bitcoin", 40.0, 10.0, buyer, "buyer-sub")
        .await
        .unwrap();

    // Assert
    assert_eq!(message, Some(Ok(Trade::Pending)));
}

#[tokio::test]
async fn should_complete_a_waiting_buy_with_a_sell() {
    // Arrange
    let (world, seller, buyer) = market().await;
    let _ = world
        .modules
        .economy
        .market
        .buy_coin("Bitcoin", 40.0, 10.0, buyer, "buyer-sub")
        .await
        .unwrap();

    // Act
    let message = world
        .modules
        .economy
        .market
        .sell_coin("Bitcoin", 40.0, 10.0, seller, "seller-sub")
        .await
        .unwrap();

    // Assert
    assert_eq!(message, Some(Ok(Trade::Complete)));
    assert_eq!(balance(&world, buyer, "Bitcoin").await, Some(40.0));
    assert_eq!(money(&world, "buyer-bank").await, 99_600.0);
    assert!(offers(&world, "Bitcoin", Side::Buy).await.is_empty());
    assert!(offers(&world, "Bitcoin", Side::Sell).await.is_empty());
}

#[tokio::test]
async fn should_skip_a_buy_offer_its_buyer_can_no_longer_pay() {
    // Arrange
    let (world, seller, buyer) = market().await;
    let _ = world
        .modules
        .economy
        .market
        .buy_coin("Bitcoin", 40.0, 10.0, buyer, "buyer-sub")
        .await
        .unwrap();

    world
        .modules
        .economy
        .bank
        .deposit("buyer-bank", -100_000.0)
        .await
        .unwrap();

    // Act
    let message = world
        .modules
        .economy
        .market
        .sell_coin("Bitcoin", 40.0, 10.0, seller, "seller-sub")
        .await
        .unwrap();

    // Assert
    assert_eq!(message, Some(Ok(Trade::Pending)));
    assert_eq!(money(&world, "buyer-bank").await, 0.0);
    assert_eq!(balance(&world, seller, "Bitcoin").await, Some(40.0));
}

#[tokio::test]
async fn should_feed_a_matched_trade_into_the_daily_price_update() {
    // Arrange
    let (world, seller, buyer) = market().await;
    let coin = world
        .modules
        .economy
        .coins
        .coin_by_name("Bitcoin")
        .await
        .unwrap()
        .unwrap();
    let _ = world
        .modules
        .economy
        .market
        .sell_coin("Bitcoin", 40.0, 10.0, seller, "seller-sub")
        .await
        .unwrap();

    // Act
    world
        .modules
        .economy
        .market
        .buy_coin("Bitcoin", 40.0, 10.0, buyer, "buyer-sub")
        .await
        .unwrap();
    world.modules.economy.market.update_prices().await.unwrap();

    // Assert: the price is the matched trades' average.
    assert_eq!(
        world
            .modules
            .economy
            .coins
            .coin_by_name("Bitcoin")
            .await
            .unwrap()
            .unwrap()
            .price,
        10.0
    );
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
        10.0
    );
}

#[tokio::test]
async fn should_give_nothing_for_a_wallet_that_is_missing() {
    // Arrange
    let (world, _, _) = market().await;

    // Act
    let message = world
        .modules
        .economy
        .market
        .buy_coin(
            "Bitcoin",
            1.0,
            1.0,
            greyhack_world_economy::WalletId::new(99),
            "buyer-sub",
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(message, None);
}

#[tokio::test]
async fn should_move_the_coins_of_a_waiting_offer_through_its_sub_wallet() {
    // Arrange
    let (world, seller, buyer) = market().await;
    let coin = world
        .modules
        .economy
        .coins
        .coin_id("Bitcoin")
        .await
        .unwrap()
        .unwrap();

    world
        .modules
        .economy
        .market
        .buy_coin("Bitcoin", 10.0, 10.0, buyer, "buyer-sub")
        .await
        .unwrap();

    // Act: the sell matches the waiting buy, whose sub-wallet takes the coins.
    world
        .modules
        .economy
        .market
        .sell_coin("Bitcoin", 10.0, 10.0, seller, "seller-sub")
        .await
        .unwrap();

    // Assert
    let subwallet = |user: &'static str| {
        let world = &world;

        async move {
            world
                .modules
                .economy
                .wallets
                .subwallet(coin, user)
                .await
                .unwrap()
                .unwrap()
                .balance
        }
    };

    assert_eq!(subwallet("buyer-sub").await, 10.0);
    assert_eq!(subwallet("seller-sub").await, 30.0);
}

#[tokio::test]
async fn should_keep_coins_a_pending_sale_offers_from_a_transfer() {
    // Arrange
    let (world, seller, _) = market().await;
    let coin = world
        .modules
        .economy
        .coins
        .coin_id("Bitcoin")
        .await
        .unwrap()
        .unwrap();

    world
        .modules
        .economy
        .market
        .sell_coin("Bitcoin", 30.0, 10.0, seller, "seller-sub")
        .await
        .unwrap();

    // Act
    let too_much = world
        .modules
        .economy
        .wallets
        .transaction(coin, "seller-sub", "buyer-sub", 20, "01/Jan/2000 - 00:00")
        .await
        .unwrap();
    let spare = world
        .modules
        .economy
        .wallets
        .transaction(coin, "seller-sub", "buyer-sub", 10, "01/Jan/2000 - 00:00")
        .await
        .unwrap();

    // Assert
    assert_eq!(
        too_much,
        Err(greyhack_world_economy::TransferFailure::SaleOfferShortfall)
    );
    assert_eq!(spare, Ok(()));
}
