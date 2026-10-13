//! Grey Hack text for world and rule failures.
//! Failure enums identify the cause; these modules provide the receiving intrinsic's exact messages.

mod admin;
mod apt;
mod computer;
mod connection;
mod crypto;
mod ctf;
mod debug;
mod device;
mod economy;
mod exploit;
mod file;
mod job;
mod mail;
mod metaxploit;
mod mission;
mod refusal;
mod service_control;
mod shell;
mod shop;
mod sniffer;

pub(in crate::game) use apt::apt_update_text;
pub(in crate::game) use connection::{
    inactive_account, lan_text, login_reaches_computer, login_text, smtp_text,
};
pub(in crate::game) use debug::{scan_text, unit_test_text};
pub(in crate::game) use economy::coin_status_text;
pub(in crate::game) use exploit::exploit_text;
use greybel_interpreter::Value;
pub(in crate::game) use metaxploit::weakness_text;
pub(in crate::game) use refusal::refusal_text;
pub(in crate::game) use shell::{invalid_password_text, invalid_shell};
pub(in crate::game) use sniffer::sniffed_text;

use crate::error::IntrinsicError;
use crate::protocol::Failure;

/// Converts `failure` into `intrinsic`'s Grey Hack return value or raised refusal error.
pub(in crate::game) fn failure_value(
    intrinsic: &'static str,
    failure: &Failure,
) -> Result<Value, IntrinsicError> {
    Ok(Value::from(failure_text(intrinsic, failure)?))
}

/// Converts `failure` into `intrinsic`'s Grey Hack return text or raised refusal error.
pub(crate) fn failure_text(
    intrinsic: &'static str,
    failure: &Failure,
) -> Result<String, IntrinsicError> {
    Ok(match failure {
        Failure::Refusal(refusal) => refusal::protocol_refusal_text(intrinsic, refusal)?,
        Failure::Market(failure) => economy::market_text(intrinsic, failure)?,
        Failure::Economy(failure) => economy::economy_text(intrinsic, failure),
        Failure::File(failure) => file::file_text(intrinsic, failure),
        Failure::Apt(failure) => apt::apt_text(failure),
        Failure::Computer(failure) => computer::computer_text(intrinsic, failure),
        Failure::Crypto(failure) => crypto::crypto_text(*failure).to_owned(),
        Failure::Mail(failure) => mail::mail_text(intrinsic, *failure),
        Failure::Ctf(failure) => ctf::ctf_text(*failure).to_owned(),
        Failure::Launch(failure) => shell::launch_text(failure),
        Failure::Login(failure) => login_text(failure),
        Failure::Shell(failure) => shell::shell_text(failure),
        Failure::Debug(failure) => debug::debug_text(failure),
        Failure::Admin(failure) => admin::admin_text(failure),
        Failure::Rshell(failure) => metaxploit::rshell_text(failure),
        Failure::Metaxploit(failure) => metaxploit::metaxploit_text(*failure).to_owned(),
        Failure::Device(failure) => device::device_text(intrinsic, failure),
        Failure::Job(failure) => job::job_text(failure),
        Failure::Alarm(failure) => device::alarm_text(failure),
        Failure::Shop(failure) => shop::shop_text(intrinsic, failure),
        Failure::Mission(failure) => mission::mission_text(failure),
        Failure::ServiceControl(failure) => {
            service_control::service_control_text(intrinsic, *failure).to_owned()
        }
    })
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
