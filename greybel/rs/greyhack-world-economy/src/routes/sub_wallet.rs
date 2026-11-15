//! Sub-wallet operations (`SubWalletIntrinsics`) with per-call coin/user lookup.

use greyhack_behaviour::verify_password;
use greyhack_world_core::ObjectTableExt as _;
use greyhack_world_router::{CycleAccess, RouteTable};

use crate::objects::{ObjectTableExt, SubWalletRef};
use crate::routes::protocol::{
    EconomyFailure, Failure, Figure, IntoProtocol as _, Outcome, ScriptFailure,
    SubWalletCheckPassword, SubWalletDeleteSubwallet, SubWalletGetBalance, SubWalletGetInfo,
    SubWalletGetUser, SubWalletLastTransaction, SubWalletMining, SubWalletSetInfo,
    SubWalletWalletUsername, Transaction,
};
use crate::routes::{OperationError, Result};
use crate::{MineJobFailure, MineJobState, SubWallet};

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: SubWalletGetBalance| {
            let world = &*ctx.modules;

            balance(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .sub_wallet(message.sub_wallet)?,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: SubWalletGetUser| {
            let _world = &*ctx.modules;

            Ok(Some(
                ctx.trees
                    .objects(call.tree)
                    .sub_wallet(message.sub_wallet)?
                    .user
                    .clone(),
            ))
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: SubWalletCheckPassword<'static>| {
            let world = &*ctx.modules;

            check_password(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .sub_wallet(message.sub_wallet)?,
                &message.password,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: SubWalletGetInfo| {
            let world = &*ctx.modules;

            info(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .sub_wallet(message.sub_wallet)?,
            )
            .await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: SubWalletSetInfo<'static>| {
            let world = &*ctx.modules;

            set_info(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .sub_wallet(message.sub_wallet)?,
                &message.info,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: SubWalletWalletUsername| {
            let world = &*ctx.modules;

            wallet_username(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .sub_wallet(message.sub_wallet)?,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: SubWalletLastTransaction| {
            let world = &*ctx.modules;

            last_transaction(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .sub_wallet(message.sub_wallet)?,
            )
            .await
        },
    );
    routes.wait::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: SubWalletMining| {
            let cycles = ctx.cycles();
            let program = call.program;
            let start = CycleAccess::change_dynamic(
                cycles,
                program,
                true,
                async move |ctx: &greyhack_world_router::RouteContext<'_, W>, call| {
                    let world = &*ctx.modules;

                    if ctx
                        .trees
                        .get_mut(call.tree)
                        .expect("the indexed tree exists")
                        .state::<MineJobState>()
                        .work
                        .is_some()
                    {
                        return Ok(Err(MineJobFailure::SecondMineJob));
                    }

                    let session = ctx
                        .trees
                        .get_mut(call.tree)
                        .expect("the indexed tree exists")
                        .session(program)
                        .expect("dispatch checked the active program");
                    let sub_wallet = &ctx
                        .trees
                        .objects(call.tree)
                        .sub_wallet(message.sub_wallet)?;
                    let start = world
                        .module::<crate::EconomyModule>()
                        .mine_jobs
                        .schedule_mine_on(
                            sub_wallet.coin,
                            &sub_wallet.user,
                            ctx.trees.objects(call.tree).session(session)?.computer,
                        )
                        .await?;

                    Ok::<_, OperationError>(start.inspect(|&job| {
                        ctx.trees
                            .get_mut(call.tree)
                            .expect("the indexed tree exists")
                            .state_mut::<MineJobState>()
                            .work = Some(job);
                    }))
                },
            )
            .await?;

            let end = match start {
                Ok(work) => CycleAccess::defer_dynamic::<
                    Result<(), MineJobFailure>,
                    W,
                    greyhack_world_core::WorldError,
                >(cycles, program, work)
                .await
                .ok_or_else(|| OperationError::Script(ScriptFailure::ProgramEnded))?,
                Err(failure) => return Ok(mine_job_failure(failure)),
            };

            match end {
                Ok(()) => Ok(Outcome::True),
                Err(failure) => Ok(mine_job_failure(failure)),
            }
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: SubWalletDeleteSubwallet| {
            let world = &*ctx.modules;

            delete(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .sub_wallet(message.sub_wallet)?,
            )
            .await
        },
    );
}

/// Applies `read` to `sub_wallet` if it exists; otherwise returns `None`.
async fn on_sub_wallet<T>(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    sub_wallet: &SubWalletRef,
    read: impl FnOnce(&SubWallet) -> T,
) -> Result<Option<T>> {
    Ok(world
        .module::<crate::EconomyModule>()
        .wallets
        .subwallet(sub_wallet.coin, &sub_wallet.user)
        .await?
        .map(|sub_wallet| read(&sub_wallet)))
}

/// Applies `change` and saves the sub-wallet like [`on_sub_wallet`], using mutable access.
async fn on_sub_wallet_mut<T>(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    sub_wallet: &SubWalletRef,
    change: impl FnOnce(&mut SubWallet) -> T,
) -> Result<Option<T>> {
    let Some(mut row) = world
        .module::<crate::EconomyModule>()
        .wallets
        .subwallet(sub_wallet.coin, &sub_wallet.user)
        .await?
    else {
        return Ok(None);
    };
    let result = change(&mut row);

    world
        .module::<crate::EconomyModule>()
        .wallets
        .save_subwallet(&row)
        .await?;

    Ok(Some(result))
}

/// Looks up the sub-wallet balance.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn balance(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    sub_wallet: &SubWalletRef,
) -> Result<Figure> {
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
pub async fn info(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    sub_wallet: &SubWalletRef,
) -> Result<Option<String>> {
    on_sub_wallet(world, sub_wallet, |sub| sub.info.clone()).await
}

/// Looks up the parent wallet username.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn wallet_username(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    sub_wallet: &SubWalletRef,
) -> Result<Option<String>> {
    on_sub_wallet(world, sub_wallet, |sub| sub.wallet_user.clone()).await
}

/// Verifies `password` against the stored sub-wallet hash.
///
/// # Errors
///
/// Returns errors from database reads.
pub async fn check_password(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
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
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    sub_wallet: &SubWalletRef,
) -> Result<Option<Transaction>> {
    Ok(world
        .module::<crate::EconomyModule>()
        .wallets
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
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
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
    Outcome::Failure(Failure::Economy(EconomyFailure::MineJob(
        failure.into_protocol(),
    )))
}

/// Deletes the referenced sub-wallet.
///
/// # Errors
///
/// Returns errors from database reads or writes.
pub async fn delete(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    sub_wallet: &SubWalletRef,
) -> Result<bool> {
    Ok(world
        .module::<crate::EconomyModule>()
        .mine_jobs
        .delete_subwallet(sub_wallet.coin, &sub_wallet.user)
        .await?)
}
