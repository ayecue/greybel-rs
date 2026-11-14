//! Economy operations of the global `admin.so` program.

use greyhack_world_router::RouteTable;
use greyhack_world_router::protocol::{AdminDeposit, AdminGrantCoins};

use crate::EconomyContext;
use crate::routes::Result;

pub(crate) fn register<W: EconomyContext + 'static>(routes: &RouteTable) {
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminGrantCoins<'static>| {
            grant_coins(
                &*ctx.modules,
                &message.wallet_user,
                &message.coin,
                message.amount,
            )
            .await
        },
    );
    routes.change::<_, _, W, greyhack_world_core::WorldError>(
        async move |ctx: &greyhack_world_router::RouteContext<'_, W>,
                    _call,
                    message: AdminDeposit<'static>| {
            Ok(ctx
                .modules
                .economy()
                .bank
                .deposit(&message.number, message.amount)
                .await?)
        },
    );
}

/// Sets the `coin` balance for `wallet_user` and reports whether the wallet exists.
async fn grant_coins(
    world: &impl EconomyContext,
    wallet_user: &str,
    coin: &str,
    amount: f64,
) -> Result<bool> {
    let economy = world.economy();
    let Some(wallet) = economy.wallets.wallet_by_user(wallet_user).await? else {
        return Ok(false);
    };

    Ok(economy.wallets.set_balance(wallet.id, coin, amount).await?)
}
