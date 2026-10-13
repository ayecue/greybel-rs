//! Shell-class intrinsics (`ShellIntrinsics`), one file per method.

use build::NATIVE as BUILD;
use connect_service::NATIVE as CONNECT_SERVICE;
use greybel_interpreter::{Native, Value, Vm};
use launch::LAUNCH;
use start_terminal::START_TERMINAL;

use crate::game::client::object::Access;
use crate::game::client::receiver;
use crate::game::client::receiver::receiver_for;

mod build;
mod connect_service;
mod host_computer;
mod launch;

pub(crate) use launch::{CachedCode, LaunchCache, launch_key};

mod ping;
mod scp;
mod start_terminal;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("host_computer", &host_computer::host_computer::NATIVE),
    ("connect_service", &CONNECT_SERVICE),
    ("ping", &ping::ping::NATIVE),
    ("scp", &scp::scp::NATIVE),
    ("launch", &LAUNCH),
    ("start_terminal", &START_TERMINAL),
    ("build", &BUILD),
];

/// Methods available on FTP shell objects (`FtpShellType`).
pub(crate) const FTP_METHODS: &[(&str, &Native)] = &[
    ("scp", &scp::scp::NATIVE),
    ("host_computer", &host_computer::host_computer::NATIVE),
];

fn shell(vm: &Vm, this: &Value, method: &str) -> Option<Access> {
    receiver_for::<receiver::Shell>(vm, this, method).cloned()
}
