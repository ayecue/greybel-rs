//! Coin operations (`CoinIntrinsics`) with per-call coin lookup.

use greyhack_world_core::ObjectTableExt as _;
use greyhack_world_router::RouteTable;

use crate::objects::{ObjectTableExt, SubWalletRef};
use crate::routes::protocol::{
    CoinCreateSubwallet, CoinGetAddress, CoinGetCycleMining, CoinGetMinedCoins, CoinGetReward,
    CoinGetSubwallet, CoinGetSubwallets, CoinResetPasswordCoin, CoinSetAddress, CoinSetCycleMining,
    CoinSetReward, CoinTransaction, IntoProtocol as _, Outcome,
};

mod access;
use self::access::{on_coin, online_coin, setting_result, transaction};

pub(crate) fn register<
    W: greyhack_world_router::WorldContext<Computer = greyhack_world_core::ComputerId> + 'static,
>(
    routes: &RouteTable,
) {
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CoinGetAddress| {
            let world = &*ctx.modules;
            let coin = online_coin(
                world,
                &ctx.trees.objects(call.tree),
                &ctx.terminal_computers(call),
                message.coin,
            )
            .await?;

            on_coin(world, coin, |coin| coin.address.clone()).await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CoinSetAddress<'static>| {
            let world = &*ctx.modules;
            let coin = online_coin(
                world,
                &ctx.trees.objects(call.tree),
                &ctx.terminal_computers(call),
                message.coin,
            )
            .await?;

            setting_result(
                world
                    .module::<crate::EconomyModule>()
                    .coins
                    .set_coin_address(coin, &message.address)
                    .await?,
            )
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CoinGetMinedCoins| {
            let world = &*ctx.modules;
            let coin = online_coin(
                world,
                &ctx.trees.objects(call.tree),
                &ctx.terminal_computers(call),
                message.coin,
            )
            .await?;

            on_coin(world, coin, |coin| coin.mined_coins).await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CoinGetReward| {
            let world = &*ctx.modules;
            let coin = online_coin(
                world,
                &ctx.trees.objects(call.tree),
                &ctx.terminal_computers(call),
                message.coin,
            )
            .await?;

            on_coin(world, coin, |coin| coin.reward).await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CoinSetReward| {
            let world = &*ctx.modules;
            let coin = online_coin(
                world,
                &ctx.trees.objects(call.tree),
                &ctx.terminal_computers(call),
                message.coin,
            )
            .await?;

            setting_result(
                world
                    .module::<crate::EconomyModule>()
                    .coins
                    .set_coin_reward(coin, message.reward)
                    .await?,
            )
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CoinGetCycleMining| {
            let world = &*ctx.modules;
            let coin = online_coin(
                world,
                &ctx.trees.objects(call.tree),
                &ctx.terminal_computers(call),
                message.coin,
            )
            .await?;

            on_coin(world, coin, |coin| coin.cycle_hours).await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CoinSetCycleMining| {
            let world = &*ctx.modules;
            let coin = online_coin(
                world,
                &ctx.trees.objects(call.tree),
                &ctx.terminal_computers(call),
                message.coin,
            )
            .await?;

            setting_result(
                world
                    .module::<crate::EconomyModule>()
                    .coins
                    .set_coin_cycle_hours(coin, message.cycle_mining)
                    .await?,
            )
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CoinCreateSubwallet<'static>| {
            let world = &*ctx.modules;
            let coin = online_coin(
                world,
                &ctx.trees.objects(call.tree),
                &ctx.terminal_computers(call),
                message.coin,
            )
            .await?;

            Ok(
                match world
                    .module::<crate::EconomyModule>()
                    .coins
                    .create_coin_subwallet(
                        coin,
                        &message.wallet,
                        &message.pin,
                        &message.user,
                        &message.password,
                    )
                    .await?
                {
                    Ok(true) => Outcome::True,
                    Ok(false) => Outcome::Null,
                    Err(failure) => Outcome::Failure(failure.into_protocol()),
                },
            )
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CoinGetSubwallet<'static>| {
            let world = &*ctx.modules;
            let coin = online_coin(
                world,
                &ctx.trees.objects(call.tree),
                &ctx.terminal_computers(call),
                message.coin,
            )
            .await?;
            let has_subwallet = world
                .module::<crate::EconomyModule>()
                .wallets
                .subwallet(coin, &message.user)
                .await?
                .is_some();

            Ok(has_subwallet.then(|| {
                ctx.trees.objects(call.tree).add_sub_wallet(SubWalletRef {
                    coin,
                    user: message.user.to_string(),
                })
            }))
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CoinGetSubwallets| {
            let world = &*ctx.modules;
            let coin = online_coin(
                world,
                &ctx.trees.objects(call.tree),
                &ctx.terminal_computers(call),
                message.coin,
            )
            .await?;

            Ok(world
                .module::<crate::EconomyModule>()
                .wallets
                .subwallets(coin)
                .await?
                .into_iter()
                .map(|sub| {
                    ctx.trees.objects(call.tree).add_sub_wallet(SubWalletRef {
                        coin,
                        user: sub.user,
                    })
                })
                .collect())
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CoinTransaction<'static>| {
            let world = &*ctx.modules;
            let coin = online_coin(
                world,
                &ctx.trees.objects(call.tree),
                &ctx.terminal_computers(call),
                message.coin,
            )
            .await?;

            transaction(
                world,
                coin,
                &message.origin,
                &message.destination,
                message.amount,
            )
            .await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: CoinResetPasswordCoin<'static>| {
            let world = &*ctx.modules;
            let coin = online_coin(
                world,
                &ctx.trees.objects(call.tree),
                &ctx.terminal_computers(call),
                message.coin,
            )
            .await?;

            Ok(
                match world
                    .module::<crate::EconomyModule>()
                    .coins
                    .reset_coin_password(
                        coin,
                        &ctx.trees
                            .objects(call.tree)
                            .session(
                                ctx.session(call)
                                    .expect("dispatch checked the active program"),
                            )?
                            .user,
                        &message.password,
                    )
                    .await?
                {
                    Ok(()) => Outcome::True,
                    Err(failure) => Outcome::Failure(failure.into_protocol()),
                },
            )
        },
    );
}
