//! `metaxploit` library loading, scanning, session, reverse-shell, and sniffing intrinsics (`MetaExploitIntrinsics`), one file per method.

use greybel_interpreter::{Native, Value, Vm};

use crate::error::IntrinsicError;
use crate::game::client::receiver;
use crate::game::client::receiver::{receiver, receiver_for, receiver_or_crash};
use crate::protocol::ObjectId;

mod load;
mod net_use;
mod rshell_client;
mod rshell_remove;
mod rshell_server;
mod scan;
mod scan_address;
mod sniffer;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("load", &load::load::NATIVE),
    ("scan", &scan::scan::NATIVE),
    ("scan_address", &scan_address::scan_address::NATIVE),
    ("net_use", &net_use::net_use::NATIVE),
    ("rshell_client", &rshell_client::rshell_client::NATIVE),
    ("rshell_remove", &rshell_remove::rshell_remove::NATIVE),
    ("rshell_server", &rshell_server::rshell_server::NATIVE),
    ("sniffer", &sniffer::sniffer::NATIVE),
];

/// Returns the server identity of metaxploit object `this`.
fn metaxploit(vm: &Vm, this: &Value) -> Option<ObjectId> {
    receiver::<receiver::Metaxploit>(vm, this).map(|metaxploit| metaxploit.reference())
}

/// Returns the metaxploit helper, crashing where Grey Hack dereferences it without a null check.
fn metaxploit_or_crash(vm: &Vm, this: &Value) -> Result<Option<ObjectId>, IntrinsicError> {
    Ok(receiver_or_crash::<receiver::Metaxploit>(vm, this)?
        .map(|metaxploit| metaxploit.reference()))
}

/// Returns the server identity of `metaLib` `meta_lib`.
fn library(vm: &Vm, meta_lib: &Value) -> Option<ObjectId> {
    receiver_for::<receiver::MetaLib>(vm, meta_lib, "lib_name").map(|library| library.reference())
}
