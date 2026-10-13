//! Wallet balances, mining nodes, PIN, and password (`WalletIntrinsics`).

use greyhack_world::{ComputerId, WalletId, World};

use crate::Result;
use crate::protocol::{EconomyFailure, Failure, Figure, Outcome};

/// Lists coins owned by the wallet.
///
/// # Errors
///
/// Returns errors from database reads.
pub(super) async fn coins(world: &World, wallet: WalletId) -> Result<Vec<String>> {
    if world.wallet(wallet).await?.is_none() {
        return Ok(Vec::new());
    }

    Ok(world
        .balances(wallet)
        .await?
        .into_iter()
        .map(|(name, _)| name)
        .collect())
}

/// Looks up the balance of `coin`, or sums all coin balances when `coin` is empty.
///
/// # Errors
///
/// Returns errors from database reads.
pub(super) async fn balance(world: &World, wallet: WalletId, coin: &str) -> Result<Figure> {
    if world.wallet(wallet).await?.is_none() {
        return Ok(Figure::Null);
    }

    if coin.is_empty() {
        return Ok(Figure::Number(world.total_balance(wallet).await?));
    }

    Ok(world.balance(wallet, coin).await?.map_or_else(
        || Figure::Failure(Failure::Economy(EconomyFailure::NoCoinBalance)),
        Figure::Number,
    ))
}

/// Lists all world coin names.
///
/// # Errors
///
/// Returns errors from database reads.
pub(super) async fn global_coins(world: &World) -> Result<Vec<String>> {
    Ok(world
        .coins()
        .await?
        .into_iter()
        .map(|coin| coin.name)
        .collect())
}

/// Counts the wallet's active mining jobs for `coin`.
///
/// # Errors
///
/// Returns errors from database reads.
pub(super) async fn show_nodes(world: &World, wallet: WalletId, coin: &str) -> Result<Figure> {
    if world.wallet(wallet).await?.is_none() {
        return Ok(Figure::Failure(Failure::Economy(
            EconomyFailure::UnknownWallet,
        )));
    }

    let Some(coin) = world.coin_id(coin).await? else {
        return Ok(Figure::Failure(Failure::Economy(
            EconomyFailure::UnknownCoin(coin.to_owned()),
        )));
    };

    Ok(Figure::Number(f64::from(
        u32::try_from(world.count_active_nodes(wallet, coin).await?).unwrap_or(u32::MAX),
    )))
}

/// Looks up the wallet PIN.
///
/// # Errors
///
/// Returns errors from database reads.
pub(super) async fn pin(world: &World, wallet: WalletId) -> Result<Option<String>> {
    Ok(world.wallet(wallet).await?.map(|wallet| wallet.pin))
}

/// Changes the wallet password after checking `host` ownership and the once-per-day reset limit.
pub(super) async fn reset_password(
    world: &mut World,
    wallet: WalletId,
    host: ComputerId,
    password: &str,
) -> Result<Outcome> {
    Ok(
        match world.reset_wallet_password(wallet, host, password).await? {
            Ok(()) => Outcome::True,
            Err(failure) => Outcome::Failure(failure.into()),
        },
    )
}
