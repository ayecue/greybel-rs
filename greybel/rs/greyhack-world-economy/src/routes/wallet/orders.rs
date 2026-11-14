//! Buy and sell orders (`WalletIntrinsics`).

use crate::routes::Result;
use crate::routes::protocol::{
    CancelTrade, EconomyFailure, Failure, GlobalOffer, GlobalOffers, IntoProtocol as _,
    PendingTrade, TradeOutcome,
};
use crate::{EconomyContext, Side, Trade, WalletId};

/// Places a buy order for `amount` of `coin` at `price` and returns its trade state or failure.
///
/// # Errors
///
/// Returns errors from database reads.
pub(super) async fn buy_coin(
    world: &impl EconomyContext,
    wallet: WalletId,
    amount: f64,
    coin: &str,
    price: f64,
    subwallet: &str,
) -> Result<TradeOutcome> {
    Ok(trade_outcome(
        world
            .economy()
            .market
            .buy_coin(coin, amount, price, wallet, subwallet)
            .await?,
    ))
}

/// Places a sell order for `amount` of `coin` at `price` and returns its trade state or failure.
///
/// # Errors
///
/// Returns errors from database reads.
pub(super) async fn sell_coin(
    world: &impl EconomyContext,
    wallet: WalletId,
    amount: f64,
    coin: &str,
    price: f64,
    subwallet: &str,
) -> Result<TradeOutcome> {
    Ok(trade_outcome(
        world
            .economy()
            .market
            .sell_coin(coin, amount, price, wallet, subwallet)
            .await?,
    ))
}

/// Converts an order result to a trade response, using `null` for a missing wallet.
pub(super) fn trade_outcome(
    result: Option<std::result::Result<Trade, crate::MarketFailure>>,
) -> TradeOutcome {
    match result {
        None => TradeOutcome::Null,
        Some(Ok(Trade::Complete)) => TradeOutcome::Complete,
        Some(Ok(Trade::Pending)) => TradeOutcome::Pending,
        Some(Ok(Trade::Partial)) => TradeOutcome::Partial,
        Some(Err(failure)) => TradeOutcome::Failure(Failure::Market(failure.into_protocol())),
    }
}

/// Looks up the wallet's pending order for `coin`.
///
/// # Errors
///
/// Returns errors from database reads.
pub(super) async fn pending_trade(
    world: &impl EconomyContext,
    wallet: WalletId,
    coin: &str,
) -> Result<PendingTrade> {
    let Some(wallet) = world.economy().wallets.wallet(wallet).await? else {
        return Ok(PendingTrade::Null);
    };
    let Some(coin) = world.economy().coins.coin_id(coin).await? else {
        return Ok(economy_failure(EconomyFailure::UnknownCoin(
            coin.to_owned(),
        )));
    };
    let offer = world
        .economy()
        .market
        .offers_of_wallet(wallet.id)
        .await?
        .into_iter()
        .find(|offer| offer.coin == coin);

    Ok(offer.map_or_else(
        || economy_failure(EconomyFailure::NoPendingTrades),
        |offer| PendingTrade::Order {
            side: offer.side.into_protocol(),
            amount: offer.amount,
            price: offer.price,
        },
    ))
}

/// Cancels the wallet's pending order for `coin`.
///
/// # Errors
///
/// Returns errors from database reads.
pub(super) async fn cancel_pending_trade(
    world: &impl EconomyContext,
    wallet: WalletId,
    coin: &str,
) -> Result<CancelTrade> {
    if world.economy().wallets.wallet(wallet).await?.is_none() {
        return Ok(CancelTrade::Null);
    }

    if world.economy().coins.coin_id(coin).await?.is_none() {
        return Ok(CancelTrade::Failure(Failure::Economy(
            EconomyFailure::UnknownCoin(coin.to_owned()),
        )));
    }

    Ok(
        if world.economy().market.cancel_trade(coin, wallet).await? {
            CancelTrade::Success
        } else {
            CancelTrade::Failure(Failure::Economy(EconomyFailure::NoPendingTrades))
        },
    )
}

/// Lists open orders for `coin`, with buys before sells.
///
/// # Errors
///
/// Returns errors from database reads.
pub(super) async fn global_offers(world: &impl EconomyContext, coin: &str) -> Result<GlobalOffers> {
    let Some(coin) = world.economy().coins.coin_by_name(coin).await? else {
        return Ok(GlobalOffers::Failure(Failure::Economy(
            EconomyFailure::UnknownCoin(coin.to_owned()),
        )));
    };
    let mut offers = Vec::new();

    for side in [Side::Buy, Side::Sell] {
        for offer in world.economy().market.offers(coin.id, side).await? {
            let Some(wallet) = world.economy().wallets.wallet(offer.wallet).await? else {
                continue;
            };

            offers.push(GlobalOffer {
                wallet_user: wallet.user,
                side: offer.side.into_protocol(),
                amount: offer.amount,
                price: offer.price,
            });
        }
    }

    Ok(GlobalOffers::Offers(offers))
}

/// Converts an economy failure into the text-based lookup response.
pub(super) fn economy_failure(failure: EconomyFailure) -> PendingTrade {
    PendingTrade::Failure(Failure::Economy(failure))
}
