use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use crate::error::IntrinsicError;
use crate::game::classes::router;
use crate::game::client::server::{program, raise, server, settle};
use crate::protocol::PlayerGetRouter;

/// Returns the script's router without `address`, or the network router at public `address` otherwise.
#[native]
pub fn get_router(
    vm: &mut Vm,
    #[default("")] address: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    let Some(address) = address.as_str() else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = PlayerGetRouter {
        program: program(vm)?,
        address: address.to_owned().into(),
    };

    Ok(settle(
        server(vm)?.send_player_get_router(&message),
        |vm, router| match router.map_err(|failure| raise("get_router", failure))? {
            Some(router) => router::create(vm, router),
            None => Ok(Value::Null),
        },
    ))
}
