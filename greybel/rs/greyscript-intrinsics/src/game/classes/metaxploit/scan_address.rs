use greybel_interpreter::{NativeFuture, NativeOutput, Value, Vm, native};

use super::library;
use crate::error::IntrinsicError;
use crate::game::client::failure::weakness_text;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::object;
use crate::game::client::receiver::{self, receiver};
use crate::game::client::server::{program, respond, server};
use crate::protocol::{MetaxploitScanAddress, ZoneScan};

/// Returns textual weaknesses at `memAddress` in `metaLib`.
#[native(method)]
pub fn scan_address(
    vm: &mut Vm,
    this: &Value,
    #[name = "metaLib"] meta_lib: &Value,
    #[name = "memAddress"] address: &Value,
) -> Result<NativeFuture, IntrinsicError> {
    if !object::game_map(vm, this) || !object::game_map(vm, meta_lib) {
        return Ok(NativeOutput::ready(Value::Null));
    }

    let Some(address) = address.as_str() else {
        return Ok(NativeOutput::ready(Value::Null));
    };

    LoginEncryption::deny_in(vm, "scan_address")?;

    let Some(library) = library(vm, meta_lib) else {
        return Ok(NativeOutput::ready(Value::Null));
    };
    let metaxploit =
        receiver::<receiver::Metaxploit>(vm, this).map(|metaxploit| metaxploit.reference());

    let message = MetaxploitScanAddress {
        program: program(vm)?,
        metaxploit,
        library,
        address: address.into(),
    };

    Ok(respond(
        vm,
        server(vm)?.send_metaxploit_scan_address(&message),
        |_, scan: Option<ZoneScan>| scan.map(|scan| scan_text(&scan)),
    ))
}

/// Formats the weaknesses of a zone (`MetaExploitIntrinsics.scan_address`). Unknown addresses retain only
/// the prelude.
fn scan_text(scan: &ZoneScan) -> String {
    let mut text = "decompiling source...\nsearching unsecure values...\n".to_owned();

    for weakness in &scan.weaknesses {
        text.push_str("Unsafe check: ");
        text.push_str(&weakness_text(weakness));
        text.push('\n');
    }

    if scan.weaknesses.is_empty() && scan.zone_exists {
        text.push_str("No vulnerabilities were found.");
    }

    text
}
