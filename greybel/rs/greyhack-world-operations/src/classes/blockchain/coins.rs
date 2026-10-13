//! Coin prices, history, and mined amounts (`BlockchainIntrinsics`).

use greyhack_world::{Coin, ComputerId, World};

use super::require_online;
use crate::Result;
use crate::connection::objects::Access;
use crate::protocol::{CoinPrice, EconomyFailure, Failure, Figure, PriceHistory};

/// Looks up coin `name` and reads its figure with `read`, returning connection or lookup failures.
pub(super) async fn on_coin(
    world: &World,
    blockchain: &Access,
    terminal_computers: &[ComputerId],
    name: &str,
    read: impl AsyncFnOnce(&Coin) -> Result<f64>,
) -> Result<Figure> {
    if let Err(failure) = require_online(world, blockchain, terminal_computers).await? {
        return Ok(Figure::Failure(failure));
    }

    let Some(coin) = world.coin_by_name(name).await? else {
        return Ok(Figure::Failure(Failure::Economy(
            EconomyFailure::UnknownCoin(name.to_owned()),
        )));
    };

    Ok(Figure::Number(read(&coin).await?))
}

/// Returns the script-visible price of `name` (`BlockchainSystem.CoinValue`, [`World::visible_price`]).
pub(super) async fn coin_price(
    world: &World,
    blockchain: &Access,
    terminal_computers: &[ComputerId],
    name: &str,
) -> Result<Figure> {
    on_coin(world, blockchain, terminal_computers, name, async |coin| {
        Ok(world.visible_price(coin.id).await?.unwrap_or(coin.price))
    })
    .await
}

/// Returns the mined balance of coin `name`.
pub(super) async fn mined_amount(
    world: &World,
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
    world: &World,
    blockchain: &Access,
    terminal_computers: &[ComputerId],
    name: &str,
) -> Result<PriceHistory> {
    if let Err(failure) = require_online(world, blockchain, terminal_computers).await? {
        return Ok(PriceHistory::Failure(failure));
    }

    let Some(coin) = world.coin_by_name(name).await? else {
        return Ok(PriceHistory::Null);
    };

    Ok(PriceHistory::Prices(
        world
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
