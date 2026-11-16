//! Tests that a program's output reaches its terminal in the game's batches.

use duplex_test_support::local::run_local;
use greybel_test_support::future::block_on;
use greybel_test_support::greyhack;
use greybel_test_support::interpreter::{
    constant_clock, constant_clock_options, with_captured_stdout,
};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_core::WorldError;
use greyhack_world_operations::WorldServer;
use greyhack_world_programs::protocol::{PlayerUserMailAddress, PrintLine};
use greyhack_world_router::RouteContext;

use crate::support::scripts::run_code_in;

#[test]
fn should_replace_the_terminal_with_a_replacing_print_and_the_prints_after_it() {
    let code = "print \"one\"\nprint(\"two\", true)\nprint \"three\"\nprint(\"four\", true)\nprint \"five\"";

    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(output, "one\n(clear)\ntwo\nthree\nfour\nfive");
}

#[test]
fn should_send_the_batch_when_the_program_waits() {
    let code =
        "print(\"one\", true)\nprint \"two\"\nwait(0.1)\nprint(\"three\", true)\nprint \"four\"";

    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(output, "(clear)\none\ntwo\n(clear)\nthree\nfour");
}

#[test]
fn should_drop_what_is_not_sent_when_the_screen_is_cleared() {
    let code = "print \"one\"\nclear_screen\nprint \"two\"";

    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(output, "(clear)\ntwo");
}

#[test]
fn should_apply_route_prints_before_the_intrinsics_continuation() {
    let code = r#"
        crypto = include_lib("/lib/crypto.so")
        admin = get_greybel_util()
        r = admin.create_network({"public_ip": "9.9.9.9", "essid": "target_net", "computers": 0})
        crypto.airmon("start", "wlan0")
        crypto.aireplay(r.bssid_name, r.essid_name)
        print "continuation"
    "#;

    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);

    let handshake = output.find("Waiting for beacon frame").expect("handshake");
    let progress = output.find("Read ").expect("capture progress");
    let continuation = output.find("continuation").expect("continuation");

    assert!(handshake < progress && progress < continuation, "{output}");
}

#[test]
fn should_print_a_launched_program_into_its_launchers_batch() {
    let code = "sh = get_shell; pc = sh.host_computer\npc.touch(home_dir, \"child.src\"); pc.File(home_dir + \"/child.src\").set_content(\"print 2\")\nsh.build(home_dir + \"/child.src\", home_dir)\nprint(1, true)\nsh.launch(home_dir + \"/child\")\nprint 3";

    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(output, "(clear)\n1\n2\n3");
}

#[test]
fn should_apply_more_than_one_cycle_of_route_prints_before_the_intrinsics_continuation() {
    const PRINTS: usize = 1025;

    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");

    assert!(world.assembly.routes.remove::<PlayerUserMailAddress>());
    world.assembly.routes.read::<_, _, World, WorldError>(
        async move |ctx: &RouteContext<'_, World>, call, _message: PlayerUserMailAddress| {
            for _ in 0..PRINTS {
                ctx.print(call, PrintLine::EncryptionEnabled);
            }

            Ok(Some("player@localhost".to_owned()))
        },
    );

    let (lines, options) = with_captured_stdout(constant_clock_options());
    let user = PlayerOptions::default().user;
    let error = run_local(async {
        let server = WorldServer::new(world, constant_clock())
            .await
            .expect("a world server");

        greyhack::run_in(
            &server,
            "user_mail_address\nprint \"continuation\"",
            &user,
            true,
            options,
        )
        .await
    });

    assert_eq!(error, None);
    assert_eq!(
        lines.borrow().join("\n"),
        format!("{}continuation", "Encryption enabled...\n".repeat(PRINTS))
    );
}
