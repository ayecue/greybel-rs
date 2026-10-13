use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};
use greyhack_behaviour::is_valid_lan_ip;

use crate::error::IntrinsicError;
use crate::game::classes::router;
use crate::game::client::server::{program, raise, server, settle};
use crate::protocol::PlayerGetSwitch;

/// Returns a router object representing a switch on the player's LAN.
#[native]
pub fn get_switch(vm: &mut Vm, address: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(address) = address.as_str() else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    if !is_valid_lan_ip(address) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let message = PlayerGetSwitch {
        program: program(vm)?,
        address: address.to_owned().into(),
    };

    Ok(settle(
        server(vm)?.send_player_get_switch(&message),
        |vm, router| match router.map_err(|failure| raise("get_switch", failure))? {
            Some(router) => router::create(vm, router),
            None => Ok(Value::Null),
        },
    ))
}
