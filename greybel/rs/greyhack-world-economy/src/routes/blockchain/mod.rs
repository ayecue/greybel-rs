//! Blockchain operations (`BlockchainIntrinsics`) that require an online host computer.

use greyhack_world_core::{Access, ObjectTableExt as _};
use greyhack_world_router::RouteTable;

use crate::objects::{CoinRef, ObjectTableExt};
use crate::routes::Result;
use crate::routes::protocol::{
    BlockchainAmountMined, BlockchainCoinPrice, BlockchainCreateWallet, BlockchainDeleteCoin,
    BlockchainGetCoin, BlockchainGetCoinName, BlockchainLoginWallet, BlockchainShowHistory,
    CoinLogin, EconomyFailure, Failure, WalletLogin,
};
use crate::{ComputerId, Connectivity, WalletId};

mod coins;
mod logins;
use self::coins::{coin_price, mined_amount, show_history};
use self::logins::{create_wallet, delete_coin, get_coin, get_coin_name, login_wallet};

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: BlockchainCoinPrice<'static>| {
            let world = &*ctx.modules;

            coin_price(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .blockchain(message.blockchain)?,
                &ctx.terminal_computers(call),
                &message.coin,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: BlockchainShowHistory<'static>| {
            let world = &*ctx.modules;

            show_history(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .blockchain(message.blockchain)?,
                &ctx.terminal_computers(call),
                &message.coin,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: BlockchainAmountMined<'static>| {
            let world = &*ctx.modules;

            mined_amount(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .blockchain(message.blockchain)?,
                &ctx.terminal_computers(call),
                &message.coin,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: BlockchainGetCoin<'static>| {
            let world = &*ctx.modules;
            let access = &ctx
                .trees
                .objects(call.tree)
                .blockchain(message.blockchain)?;
            let computer = access.computer;
            let login = get_coin(
                world,
                access,
                &ctx.terminal_computers(call),
                message.user.as_deref(),
                message.password.as_deref(),
                message.coin.as_deref(),
            )
            .await?;

            Ok(match login {
                Login::Null => CoinLogin::Null,
                Login::Object(coin) => CoinLogin::Coin(
                    ctx.trees
                        .objects(call.tree)
                        .add_coin(CoinRef { coin, computer }),
                ),
                Login::Failure(failure) => CoinLogin::Failure(failure),
            })
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: BlockchainCreateWallet<'static>| {
            let world = &*ctx.modules;
            let login = create_wallet(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .blockchain(message.blockchain)?,
                ctx.trees
                    .objects(call.tree)
                    .session(
                        ctx.session(call)
                            .expect("dispatch checked the active program"),
                    )?
                    .computer,
                &ctx.terminal_computers(call),
                message.user.as_deref(),
                message.password.as_deref(),
            )
            .await?;

            Ok(wallet(&mut ctx.trees.objects(call.tree), login))
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: BlockchainLoginWallet<'static>| {
            let world = &*ctx.modules;
            let login = login_wallet(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .blockchain(message.blockchain)?,
                &ctx.terminal_computers(call),
                message.user.as_deref(),
                message.password.as_deref(),
            )
            .await?;

            Ok(wallet(&mut ctx.trees.objects(call.tree), login))
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: BlockchainDeleteCoin<'static>| {
            let world = &*ctx.modules;

            delete_coin(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .blockchain(message.blockchain)?,
                &ctx.terminal_computers(call),
                message.coin.as_deref(),
                message.user.as_deref(),
                message.password.as_deref(),
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: BlockchainGetCoinName<'static>| {
            let world = &*ctx.modules;

            get_coin_name(
                world,
                &ctx.trees
                    .objects(call.tree)
                    .blockchain(message.blockchain)?,
                &ctx.terminal_computers(call),
                &ctx.trees
                    .objects(call.tree)
                    .session(
                        ctx.session(call)
                            .expect("dispatch checked the active program"),
                    )?
                    .user,
                message.user.as_deref(),
                message.password.as_deref(),
            )
            .await
        },
    );
}

/// Converts `login` into a wallet object response.
fn wallet(
    objects: &mut greyhack_world_router::ObjectTableHandle<ComputerId>,
    login: Login<WalletId>,
) -> WalletLogin {
    match login {
        Login::Null => WalletLogin::Null,
        Login::Object(wallet) => WalletLogin::Wallet(objects.add_wallet(wallet)),
        Login::Failure(failure) => WalletLogin::Failure(failure),
    }
}

/// Result of a script coin or wallet login.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Login<T> {
    /// Required credentials were absent.
    Null,
    /// Login produced an object identity.
    Object(T),
    /// Login failure returned to the script.
    Failure(Failure),
}

/// Returns [`EconomyFailure::NoInternet`] unless the blockchain computer is online.
pub(super) async fn require_online(
    world: &impl greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId>,
    blockchain: &Access,
    terminal_computers: &[ComputerId],
) -> Result<Result<(), Failure>> {
    if world
        .module::<crate::EconomyModule>()
        .coins
        .connectivity(blockchain.computer, terminal_computers)
        .await?
        == Connectivity::Online
    {
        Ok(Ok(()))
    } else {
        Ok(Err(Failure::Economy(EconomyFailure::NoInternet)))
    }
}
