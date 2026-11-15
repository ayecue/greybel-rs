//! Coin prices, history, and mined amounts (`BlockchainIntrinsics`).

use greyhack_world_core::{Access, ComputerId};

use super::require_online;
use crate::Coin;
use crate::routes::Result;
use crate::routes::protocol::{CoinPrice, EconomyFailure, Failure, Figure, PriceHistory};

/// Looks up coin `name` and reads its figure with `read`, returning connection or lookup failures.
pub(super) async fn on_coin(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    blockchain: &Access,
    terminal_computers: &[ComputerId],
    name: &str,
    read: impl AsyncFnOnce(&Coin) -> Result<f64>,
) -> Result<Figure> {
    if let Err(failure) = require_online(world, blockchain, terminal_computers).await? {
        return Ok(Figure::Failure(failure));
    }

    let Some(coin) = world
        .module::<crate::EconomyModule>()
        .coins
        .coin_by_name(name)
        .await?
    else {
        return Ok(Figure::Failure(Failure::Economy(
            EconomyFailure::UnknownCoin(name.to_owned()),
        )));
    };

    Ok(Figure::Number(read(&coin).await?))
}

/// Returns the script-visible price of `name` (`BlockchainSystem.CoinValue`, [`World::visible_price`]).
pub(super) async fn coin_price(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    blockchain: &Access,
    terminal_computers: &[ComputerId],
    name: &str,
) -> Result<Figure> {
    on_coin(world, blockchain, terminal_computers, name, async |coin| {
        Ok(world
            .module::<crate::EconomyModule>()
            .market
            .visible_price(coin.id)
            .await?
            .unwrap_or(coin.price))
    })
    .await
}

/// Returns the mined balance of coin `name`.
pub(super) async fn mined_amount(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    blockchain: &Access,
    terminal_computers: &[ComputerId],
    name: &str,
) -> Result<Figure> {
    on_coin(world, blockchain, terminal_computers, name, async |coin| {
        Ok(coin.mined_coins)
    })
    .await
}

/// Returns price history in chronological order (`BlockchainSystem.GetHistory`).
pub(super) async fn show_history(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    blockchain: &Access,
    terminal_computers: &[ComputerId],
    name: &str,
) -> Result<PriceHistory> {
    if let Err(failure) = require_online(world, blockchain, terminal_computers).await? {
        return Ok(PriceHistory::Failure(failure));
    }

    let Some(coin) = world
        .module::<crate::EconomyModule>()
        .coins
        .coin_by_name(name)
        .await?
    else {
        return Ok(PriceHistory::Null);
    };

    Ok(PriceHistory::Prices(
        world
            .module::<crate::EconomyModule>()
            .market
            .price_history(coin.id)
            .await?
            .into_iter()
            .map(|record| CoinPrice {
                price: record.price,
                date: record.date,
            })
            .collect(),
    ))
}
