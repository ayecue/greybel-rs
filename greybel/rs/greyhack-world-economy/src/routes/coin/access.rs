//! Coin availability checks and coin changes (`CoinIntrinsics.CheckStatus`).

use greyhack_world_core::{CoinId, ComputerId};

use crate::objects::{CoinRef, ObjectTableExt};
use crate::routes::protocol::{
    EconomyFailure, Failure, IntoProtocol as _, ObjectId, Outcome, ScriptFailure,
};
use crate::routes::{OperationError, Result};
use crate::{Coin, Connectivity, EconomyFailure as WorldFailure};

/// Returns the ID of the coin object `id` after the per-method computer checks in
/// `CoinIntrinsics.CheckStatus`.
///
/// # Errors
///
/// Returns [`ScriptFailure::Refused`] for a missing or offline computer, or a database read error.
pub(super) async fn online_coin(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    objects: &greyhack_world_programs::ObjectTableHandle<ComputerId>,
    terminal_computers: &[ComputerId],
    id: ObjectId,
) -> Result<CoinId> {
    let CoinRef { coin, computer } = objects.coin(id)?;

    match world
        .module::<crate::EconomyModule>()
        .coins
        .connectivity(computer, terminal_computers)
        .await?
    {
        Connectivity::Online => Ok(coin),
        Connectivity::Missing => Err(refusal(EconomyFailure::ComputerUnavailable)),
        Connectivity::Offline => Err(refusal(EconomyFailure::NoInternet)),
    }
}

/// Applies `read` to `coin` if it still exists; otherwise returns `None`.
pub(super) async fn on_coin<T>(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    coin: CoinId,
    read: impl FnOnce(&Coin) -> T,
) -> Result<Option<T>> {
    Ok(world
        .module::<crate::EconomyModule>()
        .coins
        .coin(coin)
        .await?
        .map(|coin| read(&coin)))
}

/// Converts a coin setting result: `None` once the coin no longer exists, or a refusal.
pub(super) fn setting_result(
    result: std::result::Result<bool, WorldFailure>,
) -> Result<Option<bool>> {
    match result {
        Ok(true) => Ok(Some(true)),
        Ok(false) => Ok(None),
        Err(failure) => Err(refusal_of(failure)),
    }
}

/// Converts `failure` from `CoinIntrinsics.CheckStatus` into a refused operation.
pub(super) fn refusal(failure: EconomyFailure) -> OperationError {
    OperationError::Script(ScriptFailure::Refused(Failure::Economy(failure)))
}

/// Converts world `failure` into a refused operation.
pub(super) fn refusal_of(failure: WorldFailure) -> OperationError {
    OperationError::Script(ScriptFailure::Refused(failure.into_protocol()))
}

/// Moves `amount` from sub-wallet `origin` into `destination`.
pub(super) async fn transaction(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    coin: CoinId,
    origin: &str,
    destination: &str,
    amount: i64,
) -> Result<Outcome> {
    let date = world
        .module::<greyhack_world_core::CoreModule>()
        .world
        .clone()
        .date();

    Ok(
        match world
            .module::<crate::EconomyModule>()
            .wallets
            .transaction(coin, origin, destination, amount, &date)
            .await?
        {
            Ok(()) => Outcome::True,
            Err(failure) => Outcome::Failure(Failure::Economy(EconomyFailure::Transfer(
                failure.into_protocol(),
            ))),
        },
    )
}
