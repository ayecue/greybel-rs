//! Sub-wallet operations (`SubWalletIntrinsics`) with per-call coin/user lookup.

use std::rc::Rc;

use duplex_core::{Channel, Driver};
use greyhack_behaviour::verify_password;
use greyhack_world::{MineJobFailure, SubWallet, WorkResult, World};

use crate::connection::Connection;
use crate::connection::objects::SubWalletRef;
use crate::protocol::{
    EconomyFailure, Failure, Figure, GameServer, Outcome, ScriptFailure, SubWalletCheckPassword,
    SubWalletDeleteSubwallet, SubWalletGetBalance, SubWalletGetInfo, SubWalletGetUser,
    SubWalletLastTransaction, SubWalletMining, SubWalletSetInfo, SubWalletWalletUsername,
    Transaction,
};
use crate::{OperationError, Result};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.read(
        channel,
        async |world, objects, message: SubWalletGetBalance, _| {
            balance(world, objects.sub_wallet(message.sub_wallet)?).await
        },
    );
    connection.read(channel, async |_, objects, message: SubWalletGetUser, _| {
        Ok(Some(objects.sub_wallet(message.sub_wallet)?.user.clone()))
    });
    connection.read(
        channel,
        async |world, objects, message: SubWalletCheckPassword<'static>, _| {
            check_password(
                world,
                objects.sub_wallet(message.sub_wallet)?,
                &message.password,
            )
            .await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: SubWalletGetInfo, _| {
            info(world, objects.sub_wallet(message.sub_wallet)?).await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: SubWalletSetInfo<'static>, _| {
            set_info(
                world,
                objects.sub_wallet(message.sub_wallet)?,
                &message.info,
            )
            .await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: SubWalletWalletUsername, _| {
            wallet_username(world, objects.sub_wallet(message.sub_wallet)?).await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: SubWalletLastTransaction, _| {
            last_transaction(world, objects.sub_wallet(message.sub_wallet)?).await
        },
    );
    connection.operate(
        channel,
        async |state, connection, message: SubWalletMining| {
            let start = state
                .cycle_for(connection.id, message.program, async |world_state| {
                    let session = state.session(connection.id, message.program)?;
                    let (world, objects) = world_state.of(connection.id);

                    if state
                        .tree_mines(connection.id, message.program, world)
                        .await?
                    {
                        return Ok(Err(MineJobFailure::SecondMineJob));
                    }

                    let sub_wallet = objects.sub_wallet(message.sub_wallet)?;
                    let start = world
                        .schedule_mine_on(
                            sub_wallet.coin,
                            &sub_wallet.user,
                            objects.session(session)?.computer,
                        )
                        .await?;

                    Ok::<_, OperationError>(
                        start.map(|job| state.defer_job(connection.id, message.program, job)),
                    )
                })
                .await?;

            let end = match start {
                Ok(end) => end
                    .await
                    .map_err(|_| OperationError::Script(ScriptFailure::ProgramEnded))?,
                Err(failure) => return Ok(mine_job_failure(failure)),
            };

            match end {
                WorkResult::Complete => Ok(Outcome::True),
                WorkResult::Failure(failure) => Ok(mine_job_failure(failure)),
                WorkResult::Capture(_) | WorkResult::Cancel => {
                    Err(OperationError::Script(ScriptFailure::ProgramEnded))
                }
            }
        },
    );
    connection.change(
        channel,
        async |world, objects, message: SubWalletDeleteSubwallet, _| {
            delete(world, objects.sub_wallet(message.sub_wallet)?).await
        },
    );
}

/// Applies `read` to `sub_wallet` if it exists; otherwise returns `None`.
async fn on_sub_wallet<T>(
    world: &World,
    sub_wallet: &SubWalletRef,
    read: impl FnOnce(&SubWallet) -> T,
) -> Result<Option<T>> {
    Ok(world
        .subwallet(sub_wallet.coin, &sub_wallet.user)
        .await?
        .map(|sub_wallet| read(&sub_wallet)))
}

/// Applies `change` and saves the sub-wallet like [`on_sub_wallet`], using mutable access.
async fn on_sub_wallet_mut<T>(
    world: &mut World,
    sub_wallet: &SubWalletRef,
    change: impl FnOnce(&mut SubWallet) -> T,
) -> Result<Option<T>> {
    let Some(mut row) = world.subwallet(sub_wallet.coin, &sub_wallet.user).await? else {
        return Ok(None);
    };
    let result = change(&mut row);

    world.save_subwallet(&row).await?;

    Ok(Some(result))
}

/// Looks up the sub-wallet balance.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn balance(world: &World, sub_wallet: &SubWalletRef) -> Result<Figure> {
    Ok(on_sub_wallet(world, sub_wallet, |sub| sub.balance)
        .await?
        .map_or_else(
            || Figure::Failure(Failure::Economy(EconomyFailure::UnknownSubwallet)),
            Figure::Number,
        ))
}

/// Looks up the sub-wallet information text.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn info(world: &World, sub_wallet: &SubWalletRef) -> Result<Option<String>> {
    on_sub_wallet(world, sub_wallet, |sub| sub.info.clone()).await
}

/// Looks up the parent wallet username.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn wallet_username(world: &World, sub_wallet: &SubWalletRef) -> Result<Option<String>> {
    on_sub_wallet(world, sub_wallet, |sub| sub.wallet_user.clone()).await
}

/// Verifies `password` against the stored sub-wallet hash.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn check_password(
    world: &World,
    sub_wallet: &SubWalletRef,
    password: &str,
) -> Result<Option<bool>> {
    on_sub_wallet(world, sub_wallet, |sub| {
        verify_password(&sub.password_hash, password)
    })
    .await
}

/// Looks up the most recent sub-wallet transaction.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn last_transaction(
    world: &World,
    sub_wallet: &SubWalletRef,
) -> Result<Option<Transaction>> {
    Ok(world
        .last_transaction(sub_wallet.coin, &sub_wallet.user)
        .await?
        .map(|last| Transaction {
            dest_account: last.dest_account,
            amount: last.amount,
            direction: last.direction,
            date: last.date,
        }))
}

/// Updates the sub-wallet information text.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn set_info(
    world: &mut World,
    sub_wallet: &SubWalletRef,
    info: &str,
) -> Result<Option<bool>> {
    Ok(
        on_sub_wallet_mut(world, sub_wallet, |sub| info.clone_into(&mut sub.info))
            .await?
            .map(|()| true),
    )
}

/// Converts a mining failure into an economy outcome.
fn mine_job_failure(failure: MineJobFailure) -> Outcome {
    Outcome::Failure(Failure::Economy(EconomyFailure::MineJob(failure.into())))
}

/// Deletes the referenced sub-wallet.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn delete(world: &mut World, sub_wallet: &SubWalletRef) -> Result<bool> {
    Ok(world
        .delete_subwallet(sub_wallet.coin, &sub_wallet.user)
        .await?)
}
