use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::coin_id;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::CoinCreateSubwallet;

/// Creates a sub-wallet in this coin for `subWalletUser`/`subWalletPass`.
#[native(method)]
pub fn create_subwallet(
    vm: &mut Vm,
    this: &Value,
    #[name = "walletID"] wallet_id: &Value,
    #[name = "pin"] pin: &Value,
    #[name = "subWalletUser"] user: &Value,
    #[name = "subWalletPass"] password: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(id) = coin_id(vm, this) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let (Some(wallet_id), Some(pin), Some(user), Some(password)) = (
        wallet_id.as_str(),
        pin.as_str(),
        user.as_str(),
        password.as_str(),
    ) else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if user.is_empty() || password.is_empty() {
        return Ok(NativeOutput::ready("Error: user and password required"));
    }

    let message = CoinCreateSubwallet {
        program: program(vm)?,
        coin: id,
        wallet: wallet_id.into(),
        pin: pin.into(),
        user: user.into(),
        password: password.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_coin_create_subwallet(&message),
        |_, outcome| outcome.response("create_subwallet"),
    ))
}
