use greybel_interpreter::{NativeFuture, Value, Vm, native};

use super::{Order, order};
use crate::error::IntrinsicError;
use crate::game::client::check::proceed;
use crate::game::client::server::{program, respond, server};
use crate::protocol::WalletSellCoin;

/// Places a sell order for `coinAmount` units of `coinName` at `coinPrice` per unit.
#[native(method)]
pub fn sell_coin(
    vm: &mut Vm,
    this: &Value,
    #[name = "coinName"] coin: &Value,
    #[name = "coin_amount"] amount: &Value,
    #[name = "unit_price"] price: &Value,
    #[name = "subwalletUser"] subwallet: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Order {
        wallet,
        coin,
        amount,
        price,
        subwallet,
    } = proceed!(order(vm, this, "sell_coin", coin, amount, price, subwallet));

    let message = WalletSellCoin {
        program: program(vm)?,
        wallet,
        amount,
        coin: coin.into(),
        price,
        subwallet: subwallet.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_wallet_sell_coin(&message),
        |_, outcome| outcome.response("sell_coin"),
    ))
}
