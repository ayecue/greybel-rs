//! Compares the system logs a script leaves in the game and in the simulated copy of its network.

use std::collections::BTreeMap;

use greybel_test_support::future::block_on;
use greyhack_behaviour::{LogEntry, SYSTEM_LOG_PATH, SystemLog};
use greyhack_test_support_client::TestClient;
use greyhack_test_support_client::protocol::test_support::{
    FileSnapshot, LanNodeKind, NetworkSnapshot,
};
use greyhack_world::World;

use super::intrinsics::{
    PROGRAM_NAME, run_in_snapshot_world_async, run_interactive_in_snapshot_world_async,
};
use super::parity::{Outcome, game_error, game_outcomes, without_ack_counts, without_users};

/// What the entries a script writes are named after the comparison replaces the game's random values.
const PLAYER: &str = "<player>";
const TOKEN: &str = "<token>";

/// The system log of a computer and the minute of its date (`Fichero.GetTimestamp`; the game and the world date a
/// log from its last entry).
#[derive(Debug, PartialEq, Eq)]
pub struct NodeLog {
    pub log: SystemLog,
    pub minute: u64,
}

/// The logs of the computers and routers of a network, by LAN address.
pub type LogService = BTreeMap<String, NodeLog>;

/// The ID of the player and the trace token that the entries a script adds name.
#[derive(Debug, Default, PartialEq, Eq)]
struct Identity {
    players: Vec<String>,
    tokens: Vec<String>,
}

/// `log` with the player's ID and trace token replaced in the entries that are not `old`, after checking their shape,
/// which is the game's: an MD5 and a GUID. The game draws them for its player, and the world derives its own. The old
/// entries are what the network already had, which both sides keep as they were: the game's token changes with each
/// player it creates (`Database.CreatePlayer` draws a GUID), so the entries of earlier players name other ones.
fn normalized(mut log: SystemLog, old: &[LogEntry], identity: &mut Identity) -> SystemLog {
    for (index, entry) in log.content_log.iter_mut().enumerate() {
        if old.get(index) == Some(entry) {
            continue;
        }

        assert!(
            entry.player_net_id.len() == 32
                && entry
                    .player_net_id
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase()),
            "the player's ID is an MD5: {:?}",
            entry.player_net_id
        );
        assert!(
            entry.token_trace.len() == 36 && entry.token_trace.matches('-').count() == 4,
            "the trace token is a GUID: {:?}",
            entry.token_trace
        );

        if !identity.players.contains(&entry.player_net_id) {
            identity.players.push(entry.player_net_id.clone());
        }

        if !identity.tokens.contains(&entry.token_trace) {
            identity.tokens.push(entry.token_trace.clone());
        }

        PLAYER.clone_into(&mut entry.player_net_id);
        TOKEN.clone_into(&mut entry.token_trace);
    }

    log
}

fn log_file(files: &[FileSnapshot]) -> Option<&FileSnapshot> {
    files
        .iter()
        .find(|file| file.path == SYSTEM_LOG_PATH && !file.folder)
}

/// The entries each log has by LAN address, as they were before a script added to them.
pub type Entries = BTreeMap<String, Vec<LogEntry>>;

/// The entries of each system log the game has in `snapshot`.
pub fn log_entries(snapshot: &NetworkSnapshot) -> Entries {
    snapshot
        .nodes
        .iter()
        .filter_map(|node| {
            let files = match &node.node {
                LanNodeKind::Router(router) => &router.files,
                LanNodeKind::Computer(computer) => &computer.files,
                LanNodeKind::Hub => return None,
            };
            let log = SystemLog::parse(&log_file(files)?.content)
                .expect("the game's log is a system log");

            Some((node.local_ip.clone(), log.content_log))
        })
        .collect()
}

/// The system logs the game has in `snapshot`, with the `old` entries each had before the script.
pub fn game_logs(snapshot: &NetworkSnapshot, old: &Entries) -> LogService {
    let mut identity = Identity::default();
    let logs = snapshot
        .nodes
        .iter()
        .filter_map(|node| {
            let files = match &node.node {
                LanNodeKind::Router(router) => &router.files,
                LanNodeKind::Computer(computer) => &computer.files,
                LanNodeKind::Hub => return None,
            };
            let file = log_file(files)?;
            let log = SystemLog::parse(&file.content).expect("the game's log is a system log");

            let old = old.get(&*node.local_ip).map_or(&[][..], Vec::as_slice);

            Some((
                node.local_ip.clone(),
                NodeLog {
                    log: normalized(log, old, &mut identity),
                    minute: file.timestamp / 60,
                },
            ))
        })
        .collect();

    assert!(identity.players.len() <= 1, "{identity:?}");
    assert!(identity.tokens.len() <= 1, "{identity:?}");

    logs
}

/// The system logs `world` has for the nodes of `snapshot`, with the `old` entries each had before the script.
pub fn world_logs(world: &World, snapshot: &NetworkSnapshot, old: &Entries) -> LogService {
    let mut identity = Identity::default();
    let logs = snapshot
        .nodes
        .iter()
        .filter(|node| !matches!(node.node, LanNodeKind::Hub))
        .filter_map(|node| {
            let computer = block_on(
                world
                    .modules
                    .core
                    .networks
                    .routers
                    .computer_at(&snapshot.public_ip, &node.local_ip),
            )
            .expect("a read")?;
            let file = block_on(world.modules.core.files.lookup.file(
                computer,
                SYSTEM_LOG_PATH,
                false,
            ))
            .expect("a read")?;
            let log = SystemLog::parse(&file.data()?.content).expect("the world's log is a log");
            let old = old.get(&*node.local_ip).map_or(&[][..], Vec::as_slice);

            Some((
                node.local_ip.clone(),
                NodeLog {
                    log: normalized(log, old, &mut identity),
                    minute: file.timestamp / 60,
                },
            ))
        })
        .collect();

    assert!(identity.players.len() <= 1, "{identity:?}");
    assert!(identity.tokens.len() <= 1, "{identity:?}");

    logs
}

/// Runs `script` in the game and in the simulated copy of the game's network as it is now, asserts that both print
/// the same and leave the same system logs, with the same dates, on every computer and router of the network, and
/// returns the game's outcome and logs. Whatever random user a script's attack names is left out.
pub async fn compare_logs(client: &TestClient, user: &str, script: &str) -> (Outcome, LogService) {
    compare_typed_logs(client, user, script, &[]).await
}

/// [`compare_logs`] typing `inputs` into the terminals the script starts, as the game fixture types them after each
/// input request.
pub async fn compare_typed_logs(
    client: &TestClient,
    user: &str,
    script: &str,
    inputs: &[String],
) -> (Outcome, LogService) {
    let before = client.network_state().await.expect("the network is read");
    let (terminal, error, world) = tokio::task::LocalSet::new()
        .run_until(async {
            if inputs.is_empty() {
                run_in_snapshot_world_async(script, user, &before).await
            } else {
                run_interactive_in_snapshot_world_async(script, user, &before, inputs).await
            }
        })
        .await;
    let expected = without_users(vec![Outcome {
        script: script.to_owned(),
        failed: error.is_some(),
        output: error.is_none().then(|| without_ack_counts(&terminal)),
        error: error
            .filter(|error| *error != greybel_interpreter::RuntimeErrorKind::GameCrash.to_string()),
    }]);
    let actual = if inputs.is_empty() {
        without_users(game_outcomes(client, &[script]).await)
    } else {
        without_users(game_interactive_outcome(client, script, inputs).await)
    };
    let after = client.network_state().await.expect("the network is read");
    let old = log_entries(&before);
    let actual_logs = game_logs(&after, &old);

    assert_eq!(actual, expected, "{script}");
    assert_eq!(actual_logs, world_logs(&world, &before, &old), "{script}");

    (actual.into_iter().next().expect("one outcome"), actual_logs)
}

/// What a script prints of the date of the log of the computer it has a shell on.
pub const LOG_DATE: &str = "print shell.host_computer.File(\"/var/system.log\").date";

pub fn connect(address: &str, port: u16, user: &str, password: &str) -> String {
    format!(
        "shell = get_shell.connect_service(\"{address}\", {port}, \"{user}\", \"{password}\", \"ssh\")"
    )
}

/// The game's outcome of `script` typing `inputs` after each input request.
async fn game_interactive_outcome(
    client: &TestClient,
    script: &str,
    inputs: &[String],
) -> Vec<Outcome> {
    let result = client
        .execute_interactive(
            script,
            "/project/test.src",
            "/project",
            PROGRAM_NAME,
            inputs,
        )
        .await
        .unwrap_or_else(|error| panic!("{script}: {error}"));
    let output = result.output.clone();

    vec![Outcome {
        script: script.to_owned(),
        failed: result.failed,
        error: result.failed.then(|| game_error(&output)).flatten(),
        output: (!result.failed).then(|| without_ack_counts(&output)),
    }]
}
