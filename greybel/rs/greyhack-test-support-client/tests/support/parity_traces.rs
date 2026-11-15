//! What the trace comparisons share: the traces of the game and of the copy of its network after a script.

use greybel_test_support::future::block_on;
use greybel_test_support::greyhack::run_on_host_at;
use greybel_test_support::interpreter::constant_clock_options;
use greyhack_test_support_client::TestClient;
use greyhack_test_support_client::protocol::test_support::{
    LanNodeKind, NetworkSnapshot, TraceComputer, TracesReply,
};
use greyhack_world::World;

use super::intrinsics::PROGRAM_NAME;
use super::parity::{assert_succeeded, game_outcomes};
use super::parity_guilds::mirrored_world;
use super::remote::ServiceAccount;

/// Experience of a level-5 guild, which may invest in one company.
pub const LEVEL_FIVE: u32 = 275 * 25;

/// The first SSH computer the player's computer can log in to by its LAN address, with a root and a non-root account.
/// A hidden computer, or one behind a switch outside the player's subnet, is unreachable
/// (`NetworkLan.Ping`), and `connect_service` then answers "Connection refused. Address unreachable".
pub fn reachable_ssh_account(snapshot: &NetworkSnapshot) -> Option<ServiceAccount> {
    snapshot.nodes.iter().find_map(|node| {
        let LanNodeKind::Computer(computer) = &node.node else {
            return None;
        };

        if node.hidden || !snapshot.nodes.iter().any(|gateway| {
            gateway.local_ip == node.gateway
                && matches!(&gateway.node, LanNodeKind::Router(router) if router.kind == "ROUTER")
        }) {
            return None;
        }

        let port = computer
            .ports
            .iter()
            .find(|port| port.service == "ssh" && !port.closed)?
            .number;
        let root = computer.users.iter().find(|user| user.name == "root")?;
        let user = computer.users.iter().find(|user| user.name != "root")?;

        Some(ServiceAccount {
            address: node.local_ip.to_string(),
            port,
            root_password: root.password.to_string(),
            user: user.name.to_string(),
            password: user.password.to_string(),
        })
    })
}

/// The addresses of the computers of the traces: active, passive and the strikes.
pub type TraceService = (Vec<(String, String)>, Vec<(String, String)>, u32);

pub fn game_traces(reply: &TracesReply) -> TraceService {
    let pair = |computer: &TraceComputer| {
        (
            computer.public_ip.to_string(),
            computer.local_ip.to_string(),
        )
    };

    (
        reply.active.iter().map(pair).collect(),
        reply.passive.iter().map(pair).collect(),
        reply.strikes,
    )
}

pub fn world_traces(world: &World) -> TraceService {
    let pair = |computer| {
        (
            block_on(world.modules.core.networks.topology.public_ip(computer)).unwrap(),
            block_on(world.modules.core.networks.topology.computer(computer))
                .unwrap()
                .unwrap()
                .local_ip,
        )
    };
    let active = block_on(world.modules.traces.traces.active_traces())
        .unwrap()
        .into_iter()
        .map(|trace| pair(trace.computer))
        .collect();
    let passive = block_on(world.modules.traces.traces.passive_traces())
        .unwrap()
        .into_iter()
        .map(pair)
        .collect();

    (
        active,
        passive,
        u32::from(block_on(world.modules.traces.traces.strikes()).unwrap()),
    )
}

pub fn script(account: &ServiceAccount) -> String {
    let home = "/home/".to_owned() + &account.user;

    format!(
        "shell = get_shell.connect_service(\"{}\", {}, \"{}\", \"{}\"); computer = shell.host_computer; computer.touch(\"{home}\", \"traced.txt\"); print computer.File(\"{home}/traced.txt\").delete",
        account.address, account.port, account.user, account.password
    )
}

/// Runs the script in the game and in a copy of its network and guilds, asserts the output and the traces left are the
/// same and returns the traces.
pub async fn compare(
    client: &TestClient,
    user: &str,
    snapshot: &NetworkSnapshot,
    account: &ServiceAccount,
) -> TraceService {
    let script = script(account);
    let guilds = client
        .query_guilds(std::slice::from_ref(&snapshot.public_ip.to_string()))
        .await
        .expect("the guilds are read");
    let expected = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let world = mirrored_world(user, snapshot, &guilds);
                let path = format!("/home/{user}/{PROGRAM_NAME}");
                let run =
                    run_on_host_at(&script, world, user, true, &path, constant_clock_options());

                (run.terminal, run.error, world_traces(&run.world))
            })
            .join()
            .expect("the intrinsics run")
    });
    let actual = game_outcomes(client, &[script.as_str()]).await;

    // The game ends a script's traces in a task of its own.
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    assert_succeeded(&actual);

    let traces = game_traces(&client.query_traces().await.expect("the traces are read"));

    assert_eq!(
        actual[0].output.as_deref().map(str::trim),
        Some(expected.0.trim())
    );
    assert_eq!(actual[0].error, expected.1);
    assert_eq!(traces, expected.2);

    traces
}
