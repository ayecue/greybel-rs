use greybel_interpreter::{NativeFuture, Value, Vm, native};

use super::{Order, order};
use crate::error::IntrinsicError;
use crate::game::client::check::proceed;
use crate::game::client::server::{program, respond, server};
use crate::protocol::WalletBuyCoin;

/// Places a buy order for `coinAmount` units of `coinName` at `coinPrice` per unit.
#[native(method)]
pub fn buy_coin(
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
    } = proceed!(order(vm, this, "buy_coin", coin, amount, price, subwallet));

    let message = WalletBuyCoin {
        program: program(vm)?,
        wallet,
        amount,
        coin: coin.trim().into(),
        price,
        subwallet: subwallet.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_wallet_buy_coin(&message),
        |_, outcome| outcome.response("buy_coin"),
    ))
}
