use greybel_interpreter::{NativeFuture, Vm, native};

use crate::error::IntrinsicError;
use crate::game::client::server::{program, respond, server};
use crate::protocol::PlayerCurrentDate;

#[native]
pub fn current_date(vm: &mut Vm) -> Result<NativeFuture, IntrinsicError> {
    Ok(respond(
        vm,
        server(vm)?.send_player_current_date(&PlayerCurrentDate {
            program: program(vm)?,
        }),
        |_, date| date,
    ))
}
