//! Tests string intrinsics with the VM's regex engine (`dotnet-regex` feature).
#![cfg(feature = "dotnet-regex")]

use greyscript_intrinsics::regex::{HostRegexEngine, NativeRegexEngine};

use crate::support::regex::{REGEX_CALLS, run};

#[test]
fn should_run_the_intrinsics_on_the_default_engine() {
    let code = REGEX_CALLS;
    let printed = run(None, code);

    assert_eq!(printed, ["3", "0", "Hlelo"]);
}

#[test]
fn should_run_the_intrinsics_on_the_vms_engine() {
    let engine = HostRegexEngine::new(NativeRegexEngine::default());
    let printed = run(Some(engine), REGEX_CALLS);

    assert_eq!(printed, ["2", "0", "Hlelo"]);
}
