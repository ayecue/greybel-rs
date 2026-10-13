use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::coin_id;
use crate::error::IntrinsicError;
use crate::game::classes::sub_wallet;
use crate::game::client::server::{program, respond, server};
use crate::protocol::CoinGetSubwallet;

/// Returns the sub-wallet belonging to `subWalletUser`.
#[native(method)]
pub fn get_subwallet(
    vm: &mut Vm,
    this: &Value,
    #[name = "subWalletUser"] user: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(id), Some(user)) = (coin_id(vm, this), user.as_str()) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = CoinGetSubwallet {
        program: program(vm)?,
        coin: id,
        user: user.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_coin_get_subwallet(&message),
        |vm, sub_wallet| match sub_wallet {
            Some(sub_wallet) => sub_wallet::create(vm, sub_wallet),
            None => Ok("get_subwallet: The account does not exist".into()),
        },
    ))
}
