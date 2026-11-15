use greyhack_behaviour::{
    FileData, FileKind, LogAction, MAX_LOG_ENTRIES, PermissionSet, SYSTEM_LOG_PATH, format_date,
    parse_date,
};
use greyhack_world::{PlayerOptions, World, WorldOptions};
use greyhack_world_core::ComputerId;
use greyhack_world_libraries::NodeDraft;

use crate::support::computers;
use crate::support::system_log::{FIRST, SECOND, log_of, log_time, world_at, write};

/// An entry names its action, address, port and the game date, and the log takes its date from the entry
/// (`PlayerUtils.WriteLog`, `Archivo.SetContenido`).
#[tokio::test]
async fn should_append_an_entry_and_date_the_log_from_it() {
    // Arrange
    let world = world_at(FIRST).await;
    let player = world.handle.player();

    // Act
    world
        .modules
        .core
        .computers
        .logs
        .write_log(player, &write(LogAction::Open, "1.2.3.4"))
        .await
        .unwrap();

    world.modules.routines.set_date(SECOND);
    world
        .modules
        .core
        .computers
        .logs
        .write_log(player, &write(LogAction::Shell, "5.6.7.8"))
        .await
        .unwrap();

    // Assert
    let log = log_of(&world, player).await;
    let entries: Vec<_> = log
        .content_log
        .iter()
        .map(|entry| (entry.action, entry.ip.as_str(), entry.fecha.as_str()))
        .collect();

    assert_eq!(
        entries,
        [
            (LogAction::Open, "1.2.3.4", FIRST),
            (LogAction::Shell, "5.6.7.8", SECOND)
        ]
    );
    assert_eq!(log.content_log[0].puerto, 22);
    assert_eq!(log_time(&world, player).await, parse_date(SECOND));
}

/// The entries name the player and carry the player's trace token.
#[tokio::test]
async fn should_name_the_player_in_each_entry() {
    // Arrange
    let world = world_at(FIRST).await;
    let player = world.handle.player();

    // Act
    world
        .modules
        .core
        .computers
        .logs
        .write_log(player, &write(LogAction::Open, "1.2.3.4"))
        .await
        .unwrap();

    // Assert
    let entry = &log_of(&world, player).await.content_log[0];

    assert_eq!(
        entry.player_net_id,
        world.modules.core.computers.logs.player_net_id()
    );
    assert_eq!(
        entry.token_trace,
        world.modules.core.computers.logs.token_trace()
    );
    assert_eq!(entry.player_net_id.len(), 32);
    assert!(!entry.tutorial);
}

/// A computer without a system log gets a root-owned one of its own with the first entry.
#[tokio::test]
async fn should_create_a_missing_log_with_the_first_entry() {
    // Arrange
    let world = world_at(FIRST).await;
    let player = world.handle.player();
    let log = world
        .modules
        .core
        .files
        .lookup
        .file(player, SYSTEM_LOG_PATH, false)
        .await
        .unwrap()
        .unwrap();

    world
        .modules
        .core
        .files
        .changes
        .remove_node(&log)
        .await
        .unwrap();

    // Act
    world
        .modules
        .core
        .computers
        .logs
        .write_log(player, &write(LogAction::Remove, "1.2.3.4"))
        .await
        .unwrap();

    // Assert
    let log = world
        .modules
        .core
        .files
        .lookup
        .file(player, SYSTEM_LOG_PATH, false)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(log.owner, "root");
    assert_eq!(log.group, "root");
    assert_eq!(log.permissions.as_str(), "-rwxr-x---");
    assert_eq!(log.data().unwrap().kind, FileKind::SystemLog);
    assert!(log.data().unwrap().is_binary());
    assert_eq!(log_of(&world, player).await.content_log.len(), 1);
    assert_eq!(log.timestamp, parse_date(FIRST));
}

/// A computer without `/var` gets it with the log, as a root-owned folder others can read.
#[tokio::test]
async fn should_create_a_missing_var_folder() {
    // Arrange
    let world = world_at(FIRST).await;
    let player = world.handle.player();
    let var = world
        .modules
        .core
        .files
        .lookup
        .folder(player, "/var", false)
        .await
        .unwrap()
        .unwrap();

    world
        .modules
        .core
        .files
        .changes
        .remove_node(&var)
        .await
        .unwrap();

    // Act
    world
        .modules
        .core
        .computers
        .logs
        .write_log(player, &write(LogAction::Open, "1.2.3.4"))
        .await
        .unwrap();

    // Assert
    let var = world
        .modules
        .core
        .files
        .lookup
        .folder(player, "/var", false)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(var.owner, "root");
    assert_eq!(var.permissions.as_str(), "drwxr-xr-x");
    assert!(var.timestamp < parse_date(FIRST));
    assert_eq!(log_of(&world, player).await.content_log.len(), 1);
}

/// A file named `system.log` that is no system log is replaced by one.
#[tokio::test]
async fn should_replace_a_file_that_is_no_system_log() {
    // Arrange
    let world = world_at(FIRST).await;
    let player = world.handle.player();
    let var = world
        .modules
        .core
        .files
        .lookup
        .folder(player, "/var", false)
        .await
        .unwrap()
        .unwrap();

    world
        .modules
        .libraries
        .generator
        .add_full_node(
            &var,
            &NodeDraft::file(
                "system.log",
                "guest",
                PermissionSet::default_for(false),
                FileData::text("not a log"),
            ),
        )
        .await
        .unwrap();

    // Act
    world
        .modules
        .core
        .computers
        .logs
        .write_log(player, &write(LogAction::Open, "1.2.3.4"))
        .await
        .unwrap();

    // Assert
    let log = world
        .modules
        .core
        .files
        .lookup
        .file(player, SYSTEM_LOG_PATH, false)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(log.owner, "root");
    assert_eq!(log.data().unwrap().kind, FileKind::SystemLog);
    assert_eq!(log_of(&world, player).await.content_log.len(), 1);
}

/// A log keeps its newest 275 entries (`LogSystem.AddLog`).
#[tokio::test]
async fn should_keep_only_the_newest_entries() {
    // Arrange
    let world = world_at(FIRST).await;
    let player = world.handle.player();

    // Act
    for index in 0..MAX_LOG_ENTRIES + 2 {
        world
            .modules
            .core
            .computers
            .logs
            .write_log(player, &write(LogAction::Open, &index.to_string()))
            .await
            .unwrap();
    }

    // Assert
    let log = log_of(&world, player).await;

    assert_eq!(log.content_log.len(), MAX_LOG_ENTRIES);
    assert_eq!(log.content_log[0].ip, "2");
}

/// A shell opened on another computer leaves `shell` there, and `bounce` on the origin and on its router
/// (`PlayerUtils.UpdateBounceLog`).
#[tokio::test]
async fn should_log_a_shell_on_the_target_the_origin_and_its_router() {
    // Arrange
    let world = world_at(FIRST).await;
    let player = world.handle.player();
    let router = world
        .modules
        .core
        .networks
        .topology
        .router_of(player)
        .await
        .unwrap()
        .unwrap();
    let target = ComputerId::new(router.id.number());
    let origin_ip = world
        .modules
        .core
        .networks
        .topology
        .public_ip(player)
        .await
        .unwrap();
    let local_ip = world
        .modules
        .core
        .networks
        .topology
        .computer(player)
        .await
        .unwrap()
        .unwrap()
        .local_ip;

    // Act
    world
        .modules
        .core
        .computers
        .logs
        .write_bounce_logs("9.9.9.9", 22, player, target)
        .await
        .unwrap();

    // Assert
    let on_target = log_of(&world, target).await.content_log;
    let on_origin = log_of(&world, player).await.content_log;

    assert_eq!(
        on_target.len(),
        2,
        "the router is the target and the origin's router"
    );
    assert_eq!(on_target[0].action, LogAction::Shell);
    assert_eq!(on_target[0].ip, origin_ip);
    assert_eq!(on_target[0].puerto, 22);
    assert_eq!(on_target[0].bounce_ip, "");
    assert_eq!(on_target[1].action, LogAction::Bounce);
    assert_eq!(on_target[1].ip, local_ip);
    assert_eq!(on_target[1].puerto, 0);
    assert_eq!(on_target[1].bounce_ip, origin_ip);
    assert_eq!(on_origin.len(), 1);
    assert_eq!(on_origin[0].action, LogAction::Bounce);
    assert_eq!(on_origin[0].ip, "9.9.9.9");
    assert_eq!(on_origin[0].bounce_ip, origin_ip);
    assert_eq!(format_date(log_time(&world, player).await), FIRST);
}

/// A shell on a computer of another network leaves its entries on that computer, the player's computer and the
/// player's router, each with the address it names.
#[tokio::test]
async fn should_log_a_shell_across_networks() {
    // Arrange
    let world = World::generate(&WorldOptions {
        seed: 7,
        networks: 1,
        player: PlayerOptions::default(),
    })
    .await
    .unwrap();

    world.modules.routines.set_date(FIRST);

    let player = world.handle.player();
    let origin_ip = world
        .modules
        .core
        .networks
        .topology
        .public_ip(player)
        .await
        .unwrap();
    let mut target = None;

    for computer in computers(&world).await {
        if world
            .modules
            .core
            .networks
            .topology
            .public_ip(computer.id)
            .await
            .unwrap()
            != origin_ip
            && !greyhack_behaviour::is_device(computer.kind)
        {
            target = Some(computer.id);

            break;
        }
    }

    let target = target.expect("a computer of the other network");
    let target_ip = world
        .modules
        .core
        .networks
        .topology
        .public_ip(target)
        .await
        .unwrap();
    let router = world
        .modules
        .core
        .networks
        .topology
        .router_of(player)
        .await
        .unwrap()
        .unwrap();
    let router = ComputerId::new(router.id.number());
    let local_ip = world
        .modules
        .core
        .networks
        .topology
        .computer(player)
        .await
        .unwrap()
        .unwrap()
        .local_ip;

    // Act
    world
        .modules
        .core
        .computers
        .logs
        .write_bounce_logs("9.9.9.9", 22, player, target)
        .await
        .unwrap();

    // Assert
    let on_target = log_of(&world, target).await.content_log;
    let on_origin = log_of(&world, player).await.content_log;
    let on_router = log_of(&world, router).await.content_log;

    assert_eq!(on_target.len(), 1);
    assert_eq!(on_target[0].action, LogAction::Shell);
    assert_eq!(on_target[0].ip, origin_ip);
    assert_eq!(on_target[0].puerto, 22);
    assert_eq!(on_origin.len(), 1);
    assert_eq!(on_origin[0].action, LogAction::Bounce);
    assert_eq!(on_origin[0].ip, "9.9.9.9");
    assert_eq!(on_origin[0].bounce_ip, target_ip);
    assert_eq!(on_router.len(), 1);
    assert_eq!(on_router[0].action, LogAction::Bounce);
    assert_eq!(on_router[0].ip, local_ip);
    assert_eq!(on_router[0].bounce_ip, target_ip);
}
