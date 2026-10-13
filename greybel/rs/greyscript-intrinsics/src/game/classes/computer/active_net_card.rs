use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::computer;
use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::{ActiveCard, ComputerActiveNetCard};

#[native(method)]
pub fn active_net_card(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    let Some(access) = computer(vm, this, "active_net_card") else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = ComputerActiveNetCard {
        program: program(vm)?,
        computer: access.reference(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_computer_active_net_card(&message),
        |_, card: Option<ActiveCard>| {
            card.map(|card| match card {
                ActiveCard::None => "NONE",
                ActiveCard::Wifi => "WIFI",
                ActiveCard::Ethernet => "ETHERNET",
            })
        },
    ))
}
