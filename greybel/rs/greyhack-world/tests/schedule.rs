use greyhack_behaviour::{FileData, FirewallRule, PermissionSet, ServiceId};
use greyhack_world::{
    ComputerOptions, MineJobFailure, MiningHardware, NetworkOptions, NodeDraft, PlayerOptions,
    Routine, WorkEnd, WorkResult, World,
};

use crate::support::schedule::mining_world;
use crate::support::{computer, computers, ended, router, save_copy, work_ends};

/// A tick runs no more due work items than its budget; the others stay due for the next.
#[tokio::test]
async fn should_run_no_more_due_work_than_the_budget() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.player();
    let at = world.time_seconds();

    for _ in 0..3 {
        world
            .schedule(
                at,
                Routine::ServiceRestart {
                    computer: player,
                    service: ServiceId::Ssh,
                },
            )
            .await
            .unwrap();
    }

    assert_eq!(world.run_due(2).await.unwrap().routines.len(), 2);
    assert_eq!(world.scheduled_routines().await.unwrap().len(), 1);
    assert_eq!(world.run_due(2).await.unwrap().routines.len(), 1);
    assert!(world.scheduled_routines().await.unwrap().is_empty());
}

/// Due work runs by its time, then by its id.
#[tokio::test]
async fn should_run_due_work_by_time_then_id() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.player();
    let now = world.time_seconds();
    let restart = |service| Routine::ServiceRestart {
        computer: player,
        service,
    };

    world.schedule(now, restart(ServiceId::Ftp)).await.unwrap();
    world
        .schedule(now - 1, restart(ServiceId::Http))
        .await
        .unwrap();
    world.schedule(now, restart(ServiceId::Ssh)).await.unwrap();

    assert_eq!(
        world.run_due(3).await.unwrap().routines,
        [
            restart(ServiceId::Http),
            restart(ServiceId::Ftp),
            restart(ServiceId::Ssh)
        ]
    );
}

/// A coin's first round runs at once; one job wins and ends, the other waits for the next round, a mining cycle on,
/// and only wins then. The jobs end like scheduled work, by id.
#[tokio::test]
async fn should_run_one_coin_round_per_cycle() {
    let (mut world, coin) = mining_world().await;
    let player = world.player();
    let alice = world
        .schedule_mine_on(coin, "alice", player)
        .await
        .unwrap()
        .unwrap();
    let bob = world
        .schedule_mine_on(coin, "bob", player)
        .await
        .unwrap()
        .unwrap();
    let report = world.tick(0).await.unwrap();
    let ends = ended(&mut world);

    assert_eq!(report.routines, [Routine::CoinRound { coin }]);
    assert!(ends.contains(&alice) != ends.contains(&bob), "one job wins");
    assert_eq!(world.mine_jobs().await.unwrap().len(), 1);

    let other = if ends.contains(&alice) { bob } else { alice };

    world.tick(3_599).await.unwrap();

    assert!(ended(&mut world).is_empty());

    world.tick(1).await.unwrap();

    assert!(ended(&mut world).contains(&other));
    assert!(world.mine_jobs().await.unwrap().is_empty());
    assert!(
        world.scheduled_routines().await.unwrap().is_empty(),
        "a coin without jobs has no round"
    );
}

/// Removing a network ends the mine jobs and the service restarts of its computers.
#[tokio::test]
async fn should_cancel_the_work_of_a_removed_network() {
    let (mut world, coin) = mining_world().await;
    let router_id = world.add_network(&NetworkOptions::default()).await.unwrap();
    let address = router(&world, router_id).await.public_ip;
    let other = world
        .add_computer_to(router_id, &ComputerOptions::default())
        .await
        .unwrap()
        .unwrap();
    let job = world
        .schedule_mine_on(coin, "alice", other)
        .await
        .unwrap()
        .unwrap();
    let restart = world
        .schedule(
            world.time_seconds() + 600,
            Routine::ServiceRestart {
                computer: other,
                service: ServiceId::Ssh,
            },
        )
        .await
        .unwrap();

    assert!(world.remove_network(&address).await.unwrap());
    assert!(world.mine_jobs().await.unwrap().is_empty());
    assert_eq!(
        world.scheduled_routines().await.unwrap().len(),
        1,
        "only the coin's round"
    );

    let ends = ended(&mut world);

    assert!(ends.contains(&job), "{ends:?}");
    assert!(ends.contains(&restart), "{ends:?}");
}

/// An NPC logs in at its randomized work start and out at the next block boundary, not every minute
/// (`NPCSchedule.NPCSchedule`; `Persona.IsLogged`).
#[tokio::test]
async fn should_switch_an_npc_at_work_block_boundaries() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.player();
    let now = world.time_seconds();
    let npc = world
        .add_npc(player, "root", "Ann", now + 10)
        .await
        .unwrap()
        .unwrap();

    assert!(world.tick(10).await.unwrap().npc_events.is_empty());

    let next = world
        .scheduled_routines()
        .await
        .unwrap()
        .into_iter()
        .find(|entry| entry.routine == Routine::NpcSwitch { npc })
        .unwrap()
        .at;

    assert!(next > now + 10);

    let report = world.tick(next - world.time_seconds()).await.unwrap();

    assert_eq!(report.npc_events.len(), 1);
    assert!(
        world
            .sessions(player)
            .await
            .unwrap()
            .contains(&"root".to_owned())
    );
    assert!(world.npc(npc).await.unwrap().unwrap().online);

    let next = world
        .scheduled_routines()
        .await
        .unwrap()
        .into_iter()
        .find(|entry| entry.routine == Routine::NpcSwitch { npc })
        .unwrap()
        .at;

    assert_eq!(
        world
            .tick(next - world.time_seconds())
            .await
            .unwrap()
            .npc_events
            .len(),
        1
    );
    assert!(!world.npc(npc).await.unwrap().unwrap().online);
}

/// Weekend schedules contain rest and leisure rather than work, so the next login is on Monday
/// (`NPCSchedule.NPCSchedule`).
#[tokio::test]
async fn should_keep_npcs_offline_through_the_weekend() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();

    world.set_date("08/Jan/2000 - 06:00");

    let player = world.player();
    let now = world.time_seconds();
    let npc = world
        .add_npc(player, "root", "Ann", now)
        .await
        .unwrap()
        .unwrap();

    assert!(world.tick(0).await.unwrap().npc_events.is_empty());
    assert!(!world.npc(npc).await.unwrap().unwrap().online);

    let next = world
        .scheduled_routines()
        .await
        .unwrap()
        .into_iter()
        .find(|entry| entry.routine == Routine::NpcSwitch { npc })
        .unwrap()
        .at;

    assert!(next >= now + 2 * 86_400, "no work starts before Monday");
    assert!(world.tick(2 * 86_400).await.unwrap().npc_events.is_empty());
}

/// An administrator entering work restores the generated network, preserving the system log as the game does
/// (`Persona.Login`; `Router.AdminRevertRouter`; `RedGlobal.AdminRevertComputer`).
#[tokio::test]
async fn should_restore_the_network_when_its_admin_enters_work() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let router_id = world
        .add_network(&NetworkOptions {
            computers: Some(3),
            ..NetworkOptions::default()
        })
        .await
        .unwrap();
    let mut npc_computer = None;

    for candidate in computers(&world).await {
        if candidate.router == Some(router_id)
            && !world.npcs_on(candidate.id).await.unwrap().is_empty()
        {
            npc_computer = Some(candidate);
            break;
        }
    }

    let computer_id = npc_computer.expect("a generated NPC computer").id;
    let npc = world.npcs_on(computer_id).await.unwrap()[0].clone();
    let original_password = world
        .user(computer_id, &npc.user)
        .await
        .unwrap()
        .unwrap()
        .password;
    let original_rules = world.firewall_rules(router_id).await.unwrap();
    let mut router_data = router(&world, router_id).await;

    world.clear_firewall(router_id).await.unwrap();
    world
        .add_firewall_rule(
            router_id,
            &FirewallRule {
                allow: false,
                port: None,
                source: None,
                destination: None,
            },
        )
        .await
        .unwrap();
    router_data.firewall_active = false;
    world.save_router(&router_data).await.unwrap();
    world
        .set_password(computer_id, &npc.user, "hacked")
        .await
        .unwrap();
    assert!(
        world
            .install_service(router_id.computer(), ServiceId::Ssh, "1.0.0")
            .await
            .unwrap()
    );

    // A missing /lib is one of the game's ShouldRestore triggers (`RedGlobal.ShouldRestore`).
    let lib = world
        .folder(computer_id, "/lib", false)
        .await
        .unwrap()
        .unwrap();

    world.remove_node(&lib).await.unwrap();

    let home = world
        .folder(computer_id, &format!("/home/{}", npc.user), false)
        .await
        .unwrap()
        .unwrap();

    world
        .add_node(
            &home,
            &NodeDraft::file(
                "hacked.txt",
                &npc.user,
                PermissionSet::default_for(false),
                FileData::text("hacked"),
            ),
        )
        .await
        .unwrap();

    let mut log = world
        .file(computer_id, "/var/system.log", false)
        .await
        .unwrap()
        .expect("the generated system log");

    log.data_mut().unwrap().content = "keep this log".to_owned();
    world.save_node(&log).await.unwrap();

    let mut restored = false;

    for _ in 0..200 {
        let now = world.time_seconds();
        let Some(next) = world.next_due_at().await.unwrap() else {
            break;
        };

        world.tick(next.saturating_sub(now)).await.unwrap();

        let password_restored = world
            .user(computer_id, &npc.user)
            .await
            .unwrap()
            .is_some_and(|user| user.password == original_password);

        if password_restored && world.firewall_rules(router_id).await.unwrap() == original_rules {
            restored = true;
            break;
        }
    }

    assert!(restored, "an administrator entered a work block");
    assert!(router(&world, router_id).await.firewall_active);
    // Only the configuration page stays on the router (`RouterConfigOS.RestorePorts`).
    assert!(
        world
            .ports(router_id.computer())
            .await
            .unwrap()
            .iter()
            .all(|port| port.service == ServiceId::Http)
    );
    assert!(
        world
            .file(
                computer_id,
                &format!("/home/{}/hacked.txt", npc.user),
                false
            )
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        world
            .content(computer_id, "/var/system.log", false)
            .await
            .unwrap()
            .unwrap(),
        "keep this log"
    );
}

/// Deleting a coin fails its round and the jobs mining it, as the game's message says.
#[tokio::test]
async fn should_fail_the_work_of_a_deleted_coin() {
    // Arrange
    let (mut world, coin) = mining_world().await;
    let player = world.player();
    let job = world
        .schedule_mine_on(coin, "alice", player)
        .await
        .unwrap()
        .unwrap();

    // Act
    let deleted = world.delete_coin("Ether").await.unwrap();

    // Assert
    let ends = work_ends(&mut world);

    assert!(deleted);
    assert!(world.scheduled_routines().await.unwrap().is_empty());
    assert!(ends.contains(&WorkEnd {
        id: job,
        result: WorkResult::Failure(MineJobFailure::CoinGone),
    }));
}

/// Deleting a sub-wallet fails the jobs mining for it.
#[tokio::test]
async fn should_fail_the_jobs_of_a_deleted_subwallet() {
    // Arrange
    let (mut world, coin) = mining_world().await;
    let player = world.player();
    let job = world
        .schedule_mine_on(coin, "alice", player)
        .await
        .unwrap()
        .unwrap();

    // Act
    world.delete_subwallet(coin, "alice").await.unwrap();

    // Assert
    assert_eq!(
        work_ends(&mut world),
        [WorkEnd {
            id: job,
            result: WorkResult::Failure(MineJobFailure::SubwalletGone),
        }]
    );
}

/// A job that wins its coin's round ends done.
#[tokio::test]
async fn should_end_a_winning_job_done() {
    // Arrange
    let (mut world, coin) = mining_world().await;
    let player = world.player();
    let job = world
        .schedule_mine_on(coin, "alice", player)
        .await
        .unwrap()
        .unwrap();

    // Act
    world.tick(0).await.unwrap();

    // Assert
    assert!(work_ends(&mut world).contains(&WorkEnd {
        id: job,
        result: WorkResult::Complete,
    }));
}

/// Removing a network removes the NPCs of its computers, whose switches stop.
#[tokio::test]
async fn should_remove_the_npcs_of_a_removed_network() {
    // Arrange
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let router_id = world.add_network(&NetworkOptions::default()).await.unwrap();
    let address = router(&world, router_id).await.public_ip;
    let other = world
        .add_computer_to(router_id, &ComputerOptions::default())
        .await
        .unwrap()
        .unwrap();
    let user = world.users(other).await.unwrap()[0].name.clone();
    let now = world.time_seconds();
    let npc = world
        .add_npc(other, &user, "Ann", now + 10)
        .await
        .unwrap()
        .unwrap();

    let database = save_copy(&mut world).await;
    let mut world = World::load(&database).await.unwrap();

    // Act
    world.remove_network(&address).await.unwrap();

    let database = save_copy(&mut world).await;
    let world = World::load(&database).await.unwrap();

    // Assert
    assert!(world.npc(npc).await.unwrap().is_none());
    assert!(world.scheduled_routines().await.unwrap().is_empty());
}

/// Rates that don't divide the time scale still pass exactly its world seconds per real second, carrying the parts.
#[tokio::test]
async fn should_pass_the_time_scale_per_real_second_at_any_tick_rate() {
    for rate in [4, 14, 20] {
        // Arrange
        let mut world = World::new(&PlayerOptions::default()).await.unwrap();
        let start = world.time_seconds();

        // Act
        world.advance_ticks(u64::from(rate) * 3, rate);

        // Assert
        assert_eq!(world.time_seconds() - start, 42, "at {rate} ticks a second");
    }
}

/// A tick passes the whole world seconds it reached, carrying the rest to the next tick.
#[tokio::test]
async fn should_carry_the_part_of_a_second_a_tick_passed() {
    // Arrange
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let start = world.time_seconds();

    // Act
    world.advance_ticks(1, 4);

    let first = world.time_seconds() - start;

    world.advance_ticks(1, 4);

    // Assert
    assert_eq!(first, 3);
    assert_eq!(world.time_seconds() - start, 7);
}

/// Setting the date drops the carried part of a second.
#[tokio::test]
async fn should_forget_the_carry_when_the_date_is_set() {
    // Arrange
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();

    world.advance_ticks(1, 4);
    world.set_date("04/Jan/2000 - 06:00");

    let start = world.time_seconds();

    // Act
    world.advance_ticks(1, 4);

    // Assert
    assert_eq!(world.time_seconds() - start, 3);
}

/// Due routines run by priority, a crashed service's restart before an NPC switching.
#[tokio::test]
async fn should_run_the_due_routines_by_priority() {
    // Arrange
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.player();
    let now = world.time_seconds();
    let restart = Routine::ServiceRestart {
        computer: player,
        service: ServiceId::Ssh,
    };
    let user = PlayerOptions::default().user;

    world
        .add_npc(player, &user, "npc", now)
        .await
        .unwrap()
        .unwrap();
    world.schedule(now, restart.clone()).await.unwrap();

    // Act
    let report = world.run_due(1).await.unwrap();

    // Assert
    assert_eq!(report.routines, [restart]);
}

/// A routine late enough runs before higher-priority ones due later, so none waits forever.
#[tokio::test]
async fn should_raise_the_priority_of_a_late_routine() {
    // Arrange
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.player();
    let now = world.time_seconds();
    let user = PlayerOptions::default().user;
    let npc = world
        .add_npc(player, &user, "npc", now)
        .await
        .unwrap()
        .unwrap();
    let switch = Routine::NpcSwitch { npc };

    world.advance_clock(2 * greyhack_behaviour::TIME_SCALE);
    world
        .schedule(
            world.time_seconds(),
            Routine::ServiceRestart {
                computer: player,
                service: ServiceId::Ssh,
            },
        )
        .await
        .unwrap();

    // Act
    let report = world.run_due(1).await.unwrap();

    // Assert
    assert_eq!(report.routines, [switch]);
}

/// A coin's round draws a wallet by its machines' total power, then one of that wallet's machines alike, whatever
/// each machine's own power (`BlockchainSystem.CheckBlocks`).
#[tokio::test]
async fn should_draw_the_winning_machine_of_a_wallet_alike() {
    // Arrange
    let (mut world, coin) = mining_world().await;
    let player = world.player();
    let router = computer(&world, player).await.router.unwrap();
    let mut machines = Vec::new();

    for power in [1.0, 9.0] {
        let machine = world
            .add_computer_to(
                router,
                &ComputerOptions {
                    mining_hardware: MiningHardware {
                        cpu_speed: 0.0,
                        cpu_cores: 0,
                        ram_model: 0,
                        gpu_hashrate: power,
                    },
                    ..ComputerOptions::default()
                },
            )
            .await
            .unwrap()
            .unwrap();

        machines.push(machine);
    }

    let mut jobs = [None, None];
    let mut wins = [0, 0];

    // Act
    for _ in 0..200 {
        for (job, &machine) in jobs.iter_mut().zip(&machines) {
            if job.is_none() {
                *job = Some(
                    world
                        .schedule_mine_on(coin, "alice", machine)
                        .await
                        .unwrap()
                        .unwrap(),
                );
            }
        }

        world.tick(3_600).await.unwrap();

        for end in work_ends(&mut world) {
            if let Some(index) = jobs.iter().position(|job| *job == Some(end.id)) {
                assert_eq!(end.result, WorkResult::Complete);
                wins[index] += 1;
                jobs[index] = None;
            }
        }
    }

    // Assert
    assert_eq!(wins[0] + wins[1], 200);
    assert!((60..=140).contains(&wins[0]), "{wins:?}");
}

/// A generated network has one to three NPCs, each switching.
#[tokio::test]
async fn should_give_a_generated_network_npcs() {
    // Arrange
    let world = World::generate(&greybel_test_support::world::world_options(7, 1))
        .await
        .unwrap();

    // Act
    let switches = world
        .scheduled_routines()
        .await
        .unwrap()
        .iter()
        .filter(|routine| matches!(routine.routine, Routine::NpcSwitch { .. }))
        .count();

    // Assert
    assert!((1..=3).contains(&switches), "{switches}");
}

/// A routine NPC schedule login leaks nothing; only a social-engineering login hands credentials to a sniffer.
#[tokio::test]
async fn should_capture_npc_credentials_only_for_a_social_engineering_login() {
    // Arrange
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.player();
    let router = computer(&world, player).await.router.unwrap();
    let server = world
        .add_computer_to(router, &ComputerOptions::default())
        .await
        .unwrap()
        .unwrap();

    world
        .install_service(server, ServiceId::Ssh, "1.0.0")
        .await
        .unwrap();

    let (user, password) = {
        let user = world
            .users(server)
            .await
            .unwrap()
            .into_iter()
            .find(|user| user.name != greyhack_behaviour::ROOT_USER)
            .expect("a user other than root");

        (user.name, user.password)
    };
    let now = world.time_seconds();

    world
        .add_npc(server, &user, "npc", now)
        .await
        .unwrap()
        .unwrap();

    let public_ip = crate::support::router(&world, router).await.public_ip;
    let local_ip = computer(&world, server).await.local_ip;
    let sniffer = world.start_sniffer(server).await.unwrap().unwrap();

    // A routine schedule login emits no packet.
    world.tick(0).await.unwrap();
    assert!(
        work_ends(&mut world)
            .into_iter()
            .all(|end| end.id != sniffer)
    );

    // Act: the social-engineering login.
    world.npc_social_login(server, &user).await.unwrap();

    // Assert
    let capture = work_ends(&mut world)
        .into_iter()
        .find(|end| end.id == sniffer)
        .expect("the sniffer captured");
    let WorkResult::Capture(packet) = capture.result else {
        panic!("a capture, got {:?}", capture.result);
    };

    assert_eq!(packet.source, public_ip);
    assert_eq!(packet.destination, local_ip);
    assert_eq!(packet.protocol, "SSH");
    assert_eq!(packet.user, user);
    assert_eq!(packet.password, password);
}

/// A social-engineering login keeps an NPC off work logged in for an hour, after which their schedule logs them out
/// (`Persona.ForceLogin`).
#[tokio::test]
async fn should_keep_an_npc_logged_in_for_an_hour_after_a_social_engineering_login() {
    // Arrange
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.player();
    let router = computer(&world, player).await.router.unwrap();
    let server = world
        .add_computer_to(router, &ComputerOptions::default())
        .await
        .unwrap()
        .unwrap();

    world
        .install_service(server, ServiceId::Ssh, "1.0.0")
        .await
        .unwrap();

    let user = world
        .users(server)
        .await
        .unwrap()
        .into_iter()
        .find(|user| user.name != greyhack_behaviour::ROOT_USER)
        .expect("a user other than root")
        .name;

    // A Sunday: the NPC isn't at work.
    world.set_date("02/Jan/2000 - 12:00");

    let now = world.time_seconds();

    world
        .add_npc(server, &user, "npc", now)
        .await
        .unwrap()
        .unwrap();
    world.tick(0).await.unwrap();

    let before = world.sessions(server).await.unwrap().contains(&user);

    // Act
    world.npc_social_login(server, &user).await.unwrap();
    world.tick(0).await.unwrap();

    let during = world.sessions(server).await.unwrap().contains(&user);

    world.tick(3_600).await.unwrap();

    let after = world.sessions(server).await.unwrap().contains(&user);

    // Assert
    assert_eq!((before, during, after), (false, true, false));
}

/// `next_due_at` reports no due time on an empty routine queue rather than failing to read a NULL.
#[tokio::test]
async fn should_report_no_due_time_when_nothing_is_scheduled() {
    let world = World::new(&PlayerOptions::default()).await.unwrap();

    assert_eq!(world.next_due_at().await.unwrap(), None);
}
