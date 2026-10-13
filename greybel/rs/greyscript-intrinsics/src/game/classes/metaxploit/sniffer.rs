use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::metaxploit_or_crash;
use crate::error::IntrinsicError;
use crate::game::client::failure::sniffed_text;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::object;
use crate::game::client::server::{program, respond, server};
use crate::game::{game, send_output};
use crate::protocol::{MetaxploitSniffer, SniffedPacket};

#[native(method)]
pub fn sniffer(
    vm: &mut Vm,
    this: &Value,
    #[default(0)]
    #[name = "encode_source"]
    encode_source: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    LoginEncryption::deny_in(vm, "sniffer")?;

    let Some(metaxploit) = metaxploit_or_crash(vm, this)? else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    let message = MetaxploitSniffer {
        program: program(vm)?,
        metaxploit,
        encode_source: encode_source.as_int() == Some(1),
        program_path: game(vm)?.session.program_path.clone().into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_metaxploit_sniffer(&message),
        |vm, packet: Option<SniffedPacket>| {
            // Grey Hack flushes output when sniffing starts; sniffing returns the captured data.
            if packet.is_some() {
                send_output(vm);
            }

            packet.as_ref().map(sniffed_text)
        },
    ))
}
