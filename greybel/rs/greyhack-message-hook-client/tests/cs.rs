//! Generates the plugin's C# protocol bindings. Set `UPDATE_CS=1` when running
//! `cargo test -p greyhack-message-hook-client --test main cs::` to regenerate them.

use duplex_cs::Module;
use greybel_test_support::protocols::{csharp, major};

#[test]
fn should_generate_the_protocol_for_the_plugin() {
    let major = major("greyhack", 1);

    let module = Module::protocol(major, "GreyHackMessageHook.Protocol");

    csharp(
        "greyhack-message-hook-proxy/src/Protocol",
        "greyhack-message-hook-client",
    )
    .assert_up_to_date("Protocol.g.cs", module);
}
