//! Coin availability checks and coin changes (`CoinIntrinsics.CheckStatus`).

use greyhack_world::{Coin, CoinId, Connectivity, EconomyFailure as WorldFailure, World};

use crate::connection::Caller;
use crate::connection::objects::{CoinRef, ObjectTable};
use crate::protocol::{EconomyFailure, Failure, ObjectId, Outcome, ScriptFailure};
use crate::{OperationError, Result};

/// Returns the ID of the coin object `id` after the per-method computer checks in
/// `CoinIntrinsics.CheckStatus`.
///
/// # Errors
///
/// Returns [`ScriptFailure::Refused`] for a missing or offline computer, or a database read error.
pub(super) async fn online_coin(
    world: &World,
    objects: &ObjectTable,
    id: ObjectId,
    caller: &Caller,
) -> Result<CoinId> {
    let CoinRef { coin, computer } = objects.coin(id)?;

    match world
        .connectivity(*computer, &caller.terminal_computers)
        .await?
    {
        Connectivity::Online => Ok(*coin),
        Connectivity::Gone => Err(refusal(EconomyFailure::ComputerUnavailable)),
        Connectivity::Offline => Err(refusal(EconomyFailure::NoInternet)),
    }
}

/// Applies `read` to `coin` if it still exists; otherwise returns `None`.
pub(super) async fn on_coin<T>(
    world: &World,
    coin: CoinId,
    read: impl FnOnce(&Coin) -> T,
) -> Result<Option<T>> {
    Ok(world.coin(coin).await?.map(|coin| read(&coin)))
}

/// Converts a coin setting result: `None` once the coin is gone, or a refusal.
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
    ScriptFailure::Refused(Failure::Economy(failure)).into()
}

/// Converts world `failure` into a refused operation.
pub(super) fn refusal_of(failure: WorldFailure) -> OperationError {
    ScriptFailure::Refused(failure.into()).into()
}

/// Moves `amount` from sub-wallet `origin` into `destination`.
pub(super) async fn transaction(
    world: &mut World,
    coin: CoinId,
    origin: &str,
    destination: &str,
    amount: i64,
) -> Result<Outcome> {
    let date = world.date();

    Ok(
        match world
            .transaction(coin, origin, destination, amount, &date)
            .await?
        {
            Ok(()) => Outcome::True,
            Err(failure) => {
                Outcome::Failure(Failure::Economy(EconomyFailure::Transfer(failure.into())))
            }
        },
    )
}
