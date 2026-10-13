//! Generates the TypeScript echo protocol modules; set `UPDATE_TS=1` to rewrite them.

use std::path::Path;

use duplex_test_support::generated::{GeneratedFolder, major};
use duplex_ts::{Module, Role};

#[test]
fn should_generate_the_echo_protocols_for_both_peers() {
    let protocols = ["echo", "offered-echo"];
    let folder = Path::new(env!("DUPLEX_TS")).join("duplex-driver-worker/test/support");
    let files = GeneratedFolder::typescript(folder.clone(), "duplex-ts");

    for name in protocols {
        let major = major(folder.join(name), 1);
        let definition = &major.latest().definition;
        let protocol = format!("./{name}.generated.js");

        files.assert_up_to_date(format!("{name}.generated.ts"), Module::protocol(major));
        for role in [Role::Client, Role::Provider] {
            let peer = &definition
                .protocol()
                .expect("the definition contains a protocol")
                .peer(role)
                .name;

            files.assert_up_to_date(
                format!("{name}.{}.generated.ts", peer.to_lowercase()),
                Module::role(major, role).with_protocol(&protocol),
            );
        }
    }
}
