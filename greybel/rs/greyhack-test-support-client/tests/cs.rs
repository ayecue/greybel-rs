use duplex_cs::Module;
use greybel_test_support::protocols::{csharp, major};

#[test]
fn should_generate_the_test_support_protocol_for_the_plugin() {
    let major = major("greyhack-test-support", 1);

    let module = Module::protocol(major, "GreyHackTestSupport.Protocol");

    csharp(
        "greyhack-test-support/plugin/Protocol",
        "greyhack-test-support-client",
    )
    .assert_up_to_date("TestSupportProtocol.g.cs", module);
}
