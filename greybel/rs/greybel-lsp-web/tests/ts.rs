//! Generates TypeScript language server protocol modules. Set `UPDATE_TS=1` when running
//! `cargo test -p greybel-lsp-web` to regenerate them.

use duplex_ts::{Module, Role};
use greybel_test_support::protocols::{PROTOCOL, major, typescript};

#[test]
fn should_generate_the_protocol() {
    let major = major("language-server", 1);

    typescript("greybel-lsp-web")
        .assert_up_to_date("src/protocol.generated.ts", Module::protocol(major));
}

#[test]
fn should_generate_the_client() {
    let major = major("language-server", 1);

    typescript("greybel-lsp-web").assert_up_to_date(
        "src/client.generated.ts",
        Module::role(major, Role::Client).with_protocol(PROTOCOL),
    );
}
