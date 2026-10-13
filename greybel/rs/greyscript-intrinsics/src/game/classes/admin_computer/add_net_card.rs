use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::address;
use crate::error::IntrinsicError;
use crate::game::client::object::ComputerAddress;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminAddNetCard;

/// Adds a Wi-Fi or Ethernet card, optionally with monitor-mode support.
#[native(method)]
pub fn add_net_card(
    vm: &mut Vm,
    this: &Value,
    wifi: &Value,
    #[name = "monitorSupport"] monitor_support: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "add_net_card";
    let Some(ComputerAddress { public_ip, lan_ip }) = address(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let heap = vm.heap();
    let message = AdminAddNetCard {
        program: program(vm)?,
        public_ip: public_ip.into(),
        lan_ip: lan_ip.into(),
        wifi: heap.is_truthy(wifi),
        monitor_support: heap.is_truthy(monitor_support),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_add_net_card(&message),
        |_, added: bool| added,
    ))
}
