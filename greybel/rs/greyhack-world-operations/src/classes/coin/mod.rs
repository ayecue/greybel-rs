//! Coin operations (`CoinIntrinsics`) with per-call coin lookup.

use std::rc::Rc;

use duplex_core::{Channel, Driver};

use crate::connection::Connection;
use crate::connection::objects::{Object, SubWalletRef};
use crate::protocol::{
    CoinCreateSubwallet, CoinGetAddress, CoinGetCycleMining, CoinGetMinedCoins, CoinGetReward,
    CoinGetSubwallet, CoinGetSubwallets, CoinResetPasswordCoin, CoinSetAddress, CoinSetCycleMining,
    CoinSetReward, CoinTransaction, GameServer, Outcome,
};

mod access;
use self::access::{on_coin, online_coin, setting_result, transaction};

pub(super) fn attach<D: Driver>(connection: &Rc<Connection>, channel: &Channel<GameServer, D>) {
    connection.read(
        channel,
        async |world, objects, message: CoinGetAddress, caller| {
            let coin = online_coin(world, objects, message.coin, caller).await?;

            on_coin(world, coin, |coin| coin.address.clone()).await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: CoinSetAddress<'static>, caller| {
            let coin = online_coin(world, objects, message.coin, caller).await?;

            setting_result(world.set_coin_address(coin, &message.address).await?)
        },
    );
    connection.read(
        channel,
        async |world, objects, message: CoinGetMinedCoins, caller| {
            let coin = online_coin(world, objects, message.coin, caller).await?;

            on_coin(world, coin, |coin| coin.mined_coins).await
        },
    );
    connection.read(
        channel,
        async |world, objects, message: CoinGetReward, caller| {
            let coin = online_coin(world, objects, message.coin, caller).await?;

            on_coin(world, coin, |coin| coin.reward).await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: CoinSetReward, caller| {
            let coin = online_coin(world, objects, message.coin, caller).await?;

            setting_result(world.set_coin_reward(coin, message.reward).await?)
        },
    );
    connection.read(
        channel,
        async |world, objects, message: CoinGetCycleMining, caller| {
            let coin = online_coin(world, objects, message.coin, caller).await?;

            on_coin(world, coin, |coin| coin.cycle_hours).await
        },
    );
    connection.change(
        channel,
        async |world, objects, message: CoinSetCycleMining, caller| {
            let coin = online_coin(world, objects, message.coin, caller).await?;

            setting_result(
                world
                    .set_coin_cycle_hours(coin, message.cycle_mining)
                    .await?,
            )
        },
    );
    connection.change(
        channel,
        async |world, objects, message: CoinCreateSubwallet<'static>, caller| {
            let coin = online_coin(world, objects, message.coin, caller).await?;

            Ok(
                match world
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
                    Err(failure) => Outcome::Failure(failure.into()),
                },
            )
        },
    );
    connection.read(
        channel,
        async |world, objects, message: CoinGetSubwallet<'static>, caller| {
            let coin = online_coin(world, objects, message.coin, caller).await?;
            let has_subwallet = world.subwallet(coin, &message.user).await?.is_some();

            Ok(has_subwallet.then(|| {
                objects.add(Object::SubWallet(SubWalletRef {
                    coin,
                    user: message.user.to_string(),
                }))
            }))
        },
    );
    connection.read(
        channel,
        async |world, objects, message: CoinGetSubwallets, caller| {
            let coin = online_coin(world, objects, message.coin, caller).await?;

            Ok(world
                .subwallets(coin)
                .await?
                .into_iter()
                .map(|sub| {
                    objects.add(Object::SubWallet(SubWalletRef {
                        coin,
                        user: sub.user,
                    }))
                })
                .collect())
        },
    );
    connection.change(
        channel,
        async |world, objects, message: CoinTransaction<'static>, caller| {
            let coin = online_coin(world, objects, message.coin, caller).await?;

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
    connection.change(
        channel,
        async |world, objects, message: CoinResetPasswordCoin<'static>, caller| {
            let coin = online_coin(world, objects, message.coin, caller).await?;

            Ok(
                match world
                    .reset_coin_password(
                        coin,
                        &objects.session(caller.session)?.user,
                        &message.password,
                    )
                    .await?
                {
                    Ok(()) => Outcome::True,
                    Err(failure) => Outcome::Failure(failure.into()),
                },
            )
        },
    );
}
