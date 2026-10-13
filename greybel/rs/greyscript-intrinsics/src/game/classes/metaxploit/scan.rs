use greybel_interpreter::{NativeFuture, NativeOutput, PrintOptions, Value, Vm, native};

use super::{library, metaxploit_or_crash};
use crate::error::IntrinsicError;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::object;
use crate::game::client::server::{program, respond, server};
use crate::game::send_output;
use crate::protocol::{LibraryScan, MetaxploitScan};

/// Warning for unscanned zones that may contain weaknesses (`MetaExploitIntrinsics.scan`).
const HIDDEN_ZONES_WARNING: &str = "**Warning: One or more memory zones with possible vulnerabilities could not be scanned. Update your Metaxploit library.**";

/// Returns `metaLib` memory addresses with weaknesses reachable by this metaxploit version.
#[native(method)]
pub fn scan(
    vm: &mut Vm,
    this: &Value,
    #[name = "metaLib"] meta_lib: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) || !object::game_map(vm, meta_lib) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    LoginEncryption::deny_in(vm, "scan")?;

    let Some(library) = library(vm, meta_lib) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let Some(metaxploit) = metaxploit_or_crash(vm, this)? else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    let message = MetaxploitScan {
        program: program(vm)?,
        metaxploit,
        library,
    };

    Ok(respond(
        vm,
        server(vm)?.send_metaxploit_scan(&message),
        |vm, scan: Option<LibraryScan>| {
            let Some(LibraryScan {
                addresses,
                hidden_zones,
            }) = scan
            else {
                return Value::Null;
            };

            if hidden_zones {
                vm.stdout()
                    .print(HIDDEN_ZONES_WARNING, PrintOptions::default());
            }

            // Grey Hack flushes output when the scan starts; the scan returns the address list.
            send_output(vm);

            Value::List(
                vm.heap_mut()
                    .add_list(addresses.into_iter().map(Value::from).collect()),
            )
        },
    ))
}
