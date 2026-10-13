use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::address;
use crate::error::IntrinsicError;
use crate::game::client::object::ComputerAddress;
use crate::game::client::server::{program, respond, server};
use crate::protocol::AdminInstallSmartphoneAdb;

/// Installs this smartphone's ADB service.
#[native(method)]
pub fn install_smartphone_adb(vm: &mut Vm, this: &Value) -> Result<NativeFuture, IntrinsicError> {
    const NAME: &str = "install_smartphone_adb";
    let Some(ComputerAddress { public_ip, lan_ip }) = address(vm, this, NAME) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let message = AdminInstallSmartphoneAdb {
        program: program(vm)?,
        public_ip: public_ip.into(),
        lan_ip: lan_ip.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_admin_install_smartphone_adb(&message),
        |_, installed: bool| installed,
    ))
}
