use greybel_interpreter::{NativeFuture, Value, Vm, native};

use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::PlayerNslookup;

/// Returns the public network address for web address `address`.
#[native]
pub fn nslookup(vm: &mut Vm, address: &Value) -> Result<NativeFuture, IntrinsicError> {
    let address = match address.as_str() {
        Some(address) if !address.is_empty() => address,
        _ => return Err(IntrinsicError::InvalidArguments("nslookup")),
    };
    let message = PlayerNslookup {
        program: program(vm)?,
        address: address.to_owned().into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_player_nslookup(&message),
        |_, address| address.unwrap_or_else(|| "IP not found".to_owned()),
    ))
}
