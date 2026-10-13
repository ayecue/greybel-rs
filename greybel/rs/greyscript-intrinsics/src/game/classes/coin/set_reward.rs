use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::coin_id;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::CoinSetReward;

#[native(method)]
pub fn set_reward(
    vm: &mut Vm,
    this: &Value,
    #[name = "coin_amount"]
    #[default(1)]
    reward: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let (Some(reward), Some(id)) = (reward.as_number(), coin_id(vm, this)) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = CoinSetReward {
        program: program(vm)?,
        coin: id,
        reward,
    };

    Ok(respond(
        vm,
        server(vm)?.send_coin_set_reward(&message),
        |_, result| result,
    ))
}
