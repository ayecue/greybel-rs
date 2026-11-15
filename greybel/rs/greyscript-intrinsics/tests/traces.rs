use greybel_test_support::future::block_on;
use greyhack_behaviour::NetworkKind;

use crate::support::scripts::run_on_host;
use crate::support::system_log::{connect_script, world_with_service};

/// Logging in to a company's computer queues it for the passive trace, and ending the script that deleted a file
/// there ends its active trace into the same queue (`PlayerUtils.ConnectService`, `FileIntrinsics.delete`,
/// `ActiveTraceSystem.CancelAllActiveTraces`).
#[test]
fn should_queue_a_company_computer_for_the_passive_trace_after_a_login_and_a_deletion() {
    let (world, target) = world_with_service();
    let row = block_on(
        world
            .modules
            .core
            .networks
            .topology
            .computer(target.computer),
    )
    .unwrap()
    .unwrap();

    block_on(
        world
            .modules
            .core
            .networks
            .settings
            .set_network_kind(row.network, NetworkKind::Bank),
    )
    .unwrap();

    let home = if target.user == "root" {
        "/root".to_owned()
    } else {
        format!("/home/{}", target.user)
    };
    let script = format!(
        "{}\ncomputer = shell.host_computer\ncomputer.touch(\"{home}\", \"traced.txt\")\nprint computer.File(\"{home}/traced.txt\").delete",
        connect_script(
            &target.address,
            target.port,
            &target.service,
            &target.user,
            &target.password
        )
    );

    let (output, error, world) = run_on_host(&script, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "shell\n");
    assert_eq!(
        block_on(world.modules.traces.traces.passive_traces()).unwrap(),
        [target.computer]
    );
    assert!(
        block_on(world.modules.traces.traces.active_traces())
            .unwrap()
            .is_empty()
    );
    assert_eq!(block_on(world.modules.traces.traces.strikes()).unwrap(), 0);
}

/// Logging in to a computer of a network the player made queues nothing (`Computer.IsSpawnedByPlayer`).
#[test]
fn should_not_queue_a_computer_of_a_rented_network() {
    let (world, target) = world_with_service();
    let row = block_on(
        world
            .modules
            .core
            .networks
            .topology
            .computer(target.computer),
    )
    .unwrap()
    .unwrap();

    block_on(
        world
            .modules
            .core
            .networks
            .settings
            .set_network_kind(row.network, NetworkKind::RentServer),
    )
    .unwrap();

    let script = connect_script(
        &target.address,
        target.port,
        &target.service,
        &target.user,
        &target.password,
    );

    let (output, error, world) = run_on_host(&script, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "shell");
    assert!(
        block_on(world.modules.traces.traces.passive_traces())
            .unwrap()
            .is_empty()
    );
}
