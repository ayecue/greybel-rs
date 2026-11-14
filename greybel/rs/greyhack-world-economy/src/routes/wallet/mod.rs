//! Wallet operations (`WalletIntrinsics`) with per-call wallet lookup.

use greyhack_world_core::ObjectTableExt as _;
use greyhack_world_router::RouteTable;

use crate::EconomyContext;
use crate::objects::ObjectTableExt;
use crate::routes::protocol::{
    WalletBuyCoin, WalletCancelPendingTrade, WalletGetBalance, WalletGetGlobalOffers,
    WalletGetPendingTrade, WalletGetPin, WalletListCoins, WalletListGlobalCoins,
    WalletResetPassword, WalletSellCoin, WalletShowNodes,
};

mod balances;
mod orders;
use self::balances::{balance, coins, global_coins, pin, reset_password, show_nodes};
use self::orders::{buy_coin, cancel_pending_trade, global_offers, pending_trade, sell_coin};

pub(crate) fn register<W: EconomyContext + 'static>(routes: &RouteTable) {
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: WalletListCoins| {
            let world = &*ctx.modules;

            coins(world, ctx.trees.objects(call.tree).wallet(message.wallet)?).await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: WalletGetBalance<'static>| {
            let world = &*ctx.modules;

            balance(
                world,
                ctx.trees.objects(call.tree).wallet(message.wallet)?,
                &message.coin,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: WalletListGlobalCoins| {
            let world = &*ctx.modules;

            ctx.trees.objects(call.tree).wallet(message.wallet)?;

            global_coins(world).await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: WalletShowNodes<'static>| {
            let world = &*ctx.modules;

            show_nodes(
                world,
                ctx.trees.objects(call.tree).wallet(message.wallet)?,
                &message.coin,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: WalletGetPin| {
            let world = &*ctx.modules;

            pin(world, ctx.trees.objects(call.tree).wallet(message.wallet)?).await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: WalletBuyCoin<'static>| {
            let world = &*ctx.modules;

            buy_coin(
                world,
                ctx.trees.objects(call.tree).wallet(message.wallet)?,
                message.amount,
                &message.coin,
                message.price,
                &message.subwallet,
            )
            .await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: WalletSellCoin<'static>| {
            let world = &*ctx.modules;

            sell_coin(
                world,
                ctx.trees.objects(call.tree).wallet(message.wallet)?,
                message.amount,
                &message.coin,
                message.price,
                &message.subwallet,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: WalletGetPendingTrade<'static>| {
            let world = &*ctx.modules;

            pending_trade(
                world,
                ctx.trees.objects(call.tree).wallet(message.wallet)?,
                &message.coin,
            )
            .await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: WalletCancelPendingTrade<'static>| {
            let world = &*ctx.modules;

            cancel_pending_trade(
                world,
                ctx.trees.objects(call.tree).wallet(message.wallet)?,
                &message.coin,
            )
            .await
        },
    );
    routes.read::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: WalletGetGlobalOffers<'static>| {
            let world = &*ctx.modules;

            ctx.trees.objects(call.tree).wallet(message.wallet)?;

            global_offers(world, &message.coin).await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    call,
                    message: WalletResetPassword<'static>| {
            let world = &*ctx.modules;

            reset_password(
                world,
                ctx.trees.objects(call.tree).wallet(message.wallet)?,
                ctx.trees
                    .objects(call.tree)
                    .session(
                        ctx.session(call)
                            .expect("dispatch checked the active program"),
                    )?
                    .computer,
                &message.password,
            )
            .await
        },
    );
}
