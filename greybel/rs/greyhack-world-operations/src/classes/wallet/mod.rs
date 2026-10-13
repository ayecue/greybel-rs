//! Wallet operations (`WalletIntrinsics`) with per-call wallet lookup.

use std::rc::Rc;

use duplex_core::{Channel, Driver};

use crate::connection::Connection;
use crate::protocol::{
    GameServer, WalletBuyCoin, WalletCancelPendingTrade, WalletGetBalance, WalletGetGlobalOffers,
    WalletGetPendingTrade, WalletGetPin, WalletListCoins, WalletListGlobalCoins,
    WalletResetPassword, WalletSellCoin, WalletShowNodes,
};

mod balances;
mod orders;
use self::balances::{balance, coins, global_coins, pin, reset_password, show_nodes};
use self::orders::{buy_coin, cancel_pending_trade, global_offers, pending_trade, sell_coin};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.read(
        channel,
        async |world, objects, message: WalletListCoins, _| {
            coins(world, *objects.wallet(message.wallet)?).await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: WalletGetBalance<'static>, _| {
            balance(world, *objects.wallet(message.wallet)?, &message.coin).await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: WalletListGlobalCoins, _| {
            objects.wallet(message.wallet)?;

            global_coins(world).await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: WalletShowNodes<'static>, _| {
            show_nodes(world, *objects.wallet(message.wallet)?, &message.coin).await
        },
    );
    connection.read(channel, async |world, objects, message: WalletGetPin, _| {
        pin(world, *objects.wallet(message.wallet)?).await
    });
    connection.change(
        channel,
        async |world, objects, message: WalletBuyCoin<'static>, _| {
            buy_coin(
                world,
                *objects.wallet(message.wallet)?,
                message.amount,
                &message.coin,
                message.price,
                &message.subwallet,
            )
            .await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: WalletSellCoin<'static>, _| {
            sell_coin(
                world,
                *objects.wallet(message.wallet)?,
                message.amount,
                &message.coin,
                message.price,
                &message.subwallet,
            )
            .await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: WalletGetPendingTrade<'static>, _| {
            pending_trade(world, *objects.wallet(message.wallet)?, &message.coin).await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: WalletCancelPendingTrade<'static>, _| {
            cancel_pending_trade(world, *objects.wallet(message.wallet)?, &message.coin).await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: WalletGetGlobalOffers<'static>, _| {
            objects.wallet(message.wallet)?;

            global_offers(world, &message.coin).await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: WalletResetPassword<'static>, caller| {
            reset_password(
                world,
                *objects.wallet(message.wallet)?,
                objects.session(caller.session)?.computer,
                &message.password,
            )
            .await
        },
    );
}
