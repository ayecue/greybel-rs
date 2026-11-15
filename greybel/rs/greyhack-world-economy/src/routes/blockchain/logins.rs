//! Coin and wallet logins, creation, and deletion (`BlockchainIntrinsics`).

use greyhack_world_core::Access;

use super::{Login, require_online};
use crate::routes::Result;
use crate::routes::protocol::{CoinName, EconomyFailure, Failure, IntoProtocol as _, Outcome};
use crate::{CoinId, ComputerId, WalletId};

/// Authenticates coin `name` with its creator `user` and `password`.
pub(super) async fn get_coin(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    blockchain: &Access,
    terminal_computers: &[ComputerId],
    user: Option<&str>,
    password: Option<&str>,
    name: Option<&str>,
) -> Result<Login<CoinId>> {
    if let Err(failure) = require_online(world, blockchain, terminal_computers).await? {
        return Ok(Login::Failure(failure));
    }

    let (Some(user), Some(password), Some(name)) = (user, password, name) else {
        return Ok(Login::Null);
    };

    Ok(world
        .module::<crate::EconomyModule>()
        .coins
        .login_coin(name, user, password)
        .await?
        .map_or_else(
            || Login::Failure(Failure::Economy(EconomyFailure::IncorrectCredentials)),
            Login::Object,
        ))
}

/// Authenticates a wallet using `user` and `password`.
pub(super) async fn login_wallet(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    blockchain: &Access,
    terminal_computers: &[ComputerId],
    user: Option<&str>,
    password: Option<&str>,
) -> Result<Login<WalletId>> {
    if let Err(failure) = require_online(world, blockchain, terminal_computers).await? {
        return Ok(Login::Failure(failure));
    }

    let (Some(user), Some(password)) = (user, password) else {
        return Ok(Login::Null);
    };

    Ok(world
        .module::<crate::EconomyModule>()
        .wallets
        .login_wallet(user, password)
        .await?
        .map_or_else(
            || Login::Failure(Failure::Economy(EconomyFailure::IncorrectCredentials)),
            Login::Object,
        ))
}

/// Creates a wallet for `user` and `password`, charging the bank account associated with the script's
/// `host` computer.
pub(super) async fn create_wallet(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    blockchain: &Access,
    host: ComputerId,
    terminal_computers: &[ComputerId],
    user: Option<&str>,
    password: Option<&str>,
) -> Result<Login<WalletId>> {
    if let Err(failure) = require_online(world, blockchain, terminal_computers).await? {
        return Ok(Login::Failure(failure));
    }

    let (Some(user), Some(password)) = (user, password) else {
        return Ok(Login::Null);
    };

    Ok(
        match world
            .module::<crate::EconomyModule>()
            .wallets
            .create_wallet_for(host, user, password)
            .await?
        {
            Ok(id) => Login::Object(id),
            Err(failure) => Login::Failure(failure.into_protocol()),
        },
    )
}

/// Deletes coin `name` after validating creator credentials.
pub(super) async fn delete_coin(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    blockchain: &Access,
    terminal_computers: &[ComputerId],
    name: Option<&str>,
    user: Option<&str>,
    password: Option<&str>,
) -> Result<Outcome> {
    if let Err(failure) = require_online(world, blockchain, terminal_computers).await? {
        return Ok(Outcome::Failure(failure));
    }

    let (Some(name), Some(user), Some(password)) = (name, user, password) else {
        return Ok(Outcome::Null);
    };

    Ok(
        match world
            .module::<crate::EconomyModule>()
            .coins
            .delete_coin_as_creator(name, user, password)
            .await?
        {
            Ok(()) => Outcome::True,
            Err(failure) => Outcome::Failure(failure.into_protocol()),
        },
    )
}

/// Returns the name of `session_user`'s coin after validating `user`/`password`, or the failure.
pub(super) async fn get_coin_name(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    blockchain: &Access,
    terminal_computers: &[ComputerId],
    session_user: &str,
    user: Option<&str>,
    password: Option<&str>,
) -> Result<CoinName> {
    if let Err(failure) = require_online(world, blockchain, terminal_computers).await? {
        return Ok(CoinName::Failure(failure));
    }

    let (Some(user), Some(password)) = (user, password) else {
        return Ok(CoinName::Null);
    };

    Ok(
        match world
            .module::<crate::EconomyModule>()
            .coins
            .creator_coin_name(session_user, user, password)
            .await?
        {
            Ok(name) => CoinName::Name(name),
            Err(failure) => CoinName::Failure(failure.into_protocol()),
        },
    )
}
