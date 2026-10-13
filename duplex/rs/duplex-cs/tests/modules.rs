use duplex_cs::Module;
use duplex_protocol::{Version, VersionHistory};
use duplex_test_support::conformance::definition;
use duplex_test_support::generated::major;

#[test]
fn should_generate_the_calculator() {
    let major = major(definition("calculator"), 1);

    let module = Module::protocol(major, "Duplex.Tests.Calculator").to_string();

    assert!(
        module.contains("public sealed class CalculatorChannel : global::Duplex.ProviderChannel"),
        "{module}"
    );
}

#[test]
#[should_panic(expected = "has a handshake")]
fn should_refuse_a_protocol_with_a_handshake() {
    let major = major(definition("login"), 1);

    let _ = Module::protocol(major, "Duplex.Tests.Login");
}

#[test]
#[should_panic(expected = "has the name of a type its payload names")]
fn should_refuse_a_variant_named_as_its_payload_type() {
    let source = "use core 1.0;\nprotocol P = \"p\" {\n    provider Server {\n        message Request { value: Reply }\n    }\n    client User {}\n}\nunion Reply {\n    Text(Text),\n}\nstruct Text {\n    text: string,\n}\n";
    let versions = VersionHistory::of_sources("p", &[(Version::new(1, 0), source)])
        .expect("the definition loads");

    let _ = Module::types(versions.major(1).expect("major 1"), "Tests").to_string();
}
