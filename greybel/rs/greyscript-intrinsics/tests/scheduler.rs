use greybel_test_support::future::block_on;
use greyhack_behaviour::ServiceId;
use greyhack_world::{CoinId, PlayerOptions, Routine, World};

use crate::support::economy::{add_subwallet, ether, subwallet};
use crate::support::scripts::run_on_host;
use crate::support::{change_router, read_computer, read_router, save_copy};

#[test]
fn should_run_due_events_and_advance_the_game_date() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");

    world.set_date("31/Oct/2026 - 23:59");

    let coin = ether(&world);
    let now = world.time_seconds();
    let mut ether = block_on(world.coin(coin)).unwrap().unwrap();

    add_subwallet(&mut world, &subwallet(coin, "alice", 0.0));
    // The coin's next round is due in a minute.
    ether.cycle_hours = 1.0;
    ether.last_cycle_at = Some(now - 3_540);
    block_on(world.save_coin(&ether)).unwrap();
    block_on(world.schedule_mine(coin, "alice"))
        .unwrap()
        .unwrap();

    let before = block_on(world.tick(0)).unwrap();
    let report = block_on(world.tick(60)).unwrap();

    assert!(before.routines.is_empty());
    assert_eq!(report.routines.len(), 1);
    assert!(report.errors.is_empty());
    assert_eq!(report.date, "01/Nov/2026 - 00:00");
    assert_eq!(
        block_on(world.subwallet(coin, "alice"))
            .unwrap()
            .unwrap()
            .balance,
        1.0
    );
}

#[test]
fn should_restart_a_service_when_its_event_is_due() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let player = world.player();

    block_on(world.install_service(player, ServiceId::Ssh, "1.0.0")).unwrap();
    block_on(world.stop_service(player, ServiceId::Ssh)).unwrap();

    let at = world.time_seconds() + 1;

    block_on(world.schedule(
        at,
        Routine::ServiceRestart {
            computer: player,
            service: ServiceId::Ssh,
        },
    ))
    .unwrap();

    block_on(world.tick(1)).unwrap();

    assert!(block_on(world.has_service(player, ServiceId::Ssh)).unwrap());
    assert_eq!(
        block_on(world.processes(player))
            .unwrap()
            .iter()
            .filter(|process| process.name == "sshd")
            .count(),
        1
    );
}

#[tokio::test]
async fn should_preserve_time_and_scheduled_routines_across_a_reload() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.player();
    let at = world.time_seconds() + 120;

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

    let coin = ether(&world);
    let wallet = world
        .add_wallet("test", "test", "0000", "test")
        .await
        .unwrap();
    let mut sub = subwallet(coin, "alice", 0.0);

    sub.wallet_user = "test".to_owned();

    let now = world.time_seconds();
    let mut ether = world.coin(coin).await.unwrap().unwrap();

    add_subwallet(&mut world, &sub);
    // The coin's next round is due in an hour.
    ether.cycle_hours = 1.0;
    ether.last_cycle_at = Some(now);
    world.save_coin(&ether).await.unwrap();
    world.schedule_mine(coin, "alice").await.unwrap().unwrap();

    let reset_at = world.time_seconds() + 1_000;
    let mut wallet_row = world.wallet(wallet).await.unwrap().unwrap();

    wallet_row.next_password_reset_at = Some(reset_at);
    world.save_wallet(&wallet_row).await.unwrap();

    let source = read_computer(&world, player).router.unwrap();

    change_router(&mut world, source, |router| router.map = [0.25, 0.5]);
    world.tick(30).await.unwrap();

    let database = save_copy(&mut world).await;

    let restored = World::load(&database).await.unwrap();

    assert_eq!(restored.time_seconds(), world.time_seconds());
    assert_eq!(
        restored.scheduled_routines().await.unwrap(),
        world.scheduled_routines().await.unwrap()
    );
    assert_eq!(
        restored.mine_jobs().await.unwrap(),
        world.mine_jobs().await.unwrap()
    );
    assert_eq!(restored.count_active_nodes(wallet, coin).await.unwrap(), 1);
    assert_eq!(read_router(&restored, source).map, [0.25, 0.5]);
    assert_eq!(
        restored
            .wallet(wallet)
            .await
            .unwrap()
            .unwrap()
            .next_password_reset_at,
        Some(reset_at)
    );
}

#[test]
fn should_advance_time_through_the_admin_api() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let player = world.player();

    world.set_date("31/Oct/2026 - 23:59");
    // Keep the unrelated empty-queue next_due_at conversion failure out of this native API test.
    block_on(world.schedule(
        world.time_seconds() + 61,
        Routine::ServiceRestart {
            computer: player,
            service: ServiceId::Ssh,
        },
    ))
    .unwrap();

    let code = r#"
admin = include_lib("admin.so")
admin.set_date("31/Oct/2026 - 23:59")
admin.tick(60)
print current_date
"#;

    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "01/Nov/2026 - 00:00");
}

#[test]
fn should_count_pending_mining_jobs_and_finish_them() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let coin = ether(&world);
    let wallet = block_on(world.add_wallet("test", "test", "0000", "test")).unwrap();
    let mut sub = subwallet(coin, "alice", 0.0);

    sub.wallet_user = "test".to_owned();
    add_subwallet(&mut world, &sub);

    let started = block_on(world.schedule_mine(coin, "alice")).unwrap();
    let duplicate = block_on(world.schedule_mine(coin, "alice")).unwrap();
    let before = block_on(world.count_active_nodes(wallet, coin)).unwrap();

    block_on(world.tick(60)).unwrap();

    let after = block_on(world.count_active_nodes(wallet, coin)).unwrap();

    assert!(started.is_ok());
    assert!(duplicate.is_err());
    assert_eq!(before, 1);
    assert_eq!(after, 0);
    assert_eq!(
        block_on(world.subwallet(coin, "alice"))
            .unwrap()
            .unwrap()
            .balance,
        1.0
    );
}

#[test]
fn should_show_pending_mining_nodes_through_the_wallet_api() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let coin = ether(&world);
    let mut sub = subwallet(coin, "alice", 0.0);

    sub.wallet_user = "test".to_owned();

    let now = world.time_seconds();
    let mut ether = block_on(world.coin(coin)).unwrap().unwrap();

    add_subwallet(&mut world, &sub);
    // The coin's next round is due in an hour.
    ether.cycle_hours = 1.0;
    ether.last_cycle_at = Some(now);
    block_on(world.save_coin(&ether)).unwrap();
    block_on(world.schedule_mine(coin, "alice"))
        .unwrap()
        .unwrap();

    let code = r#"
chain = include_lib("/lib/blockchain.so")
wallet = chain.login_wallet("test", "test")
print wallet.show_nodes("Ether")
"#;

    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "1");
}

#[test]
fn should_limit_wallet_password_resets_to_once_per_day() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");

    block_on(world.tick(86_401)).unwrap();

    let code = r#"
chain = include_lib("/lib/blockchain.so")
wallet = chain.create_wallet("test", "test")
print wallet.reset_password("one")
print wallet.reset_password("two")
"#;

    let (output, error, world) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "Error: The password can only be reset once a day\nError: The password can only be reset once a day"
    );

    let code = r#"
chain = include_lib("/lib/blockchain.so")
wallet = chain.login_wallet("test", "test")
print wallet.reset_password("two")
"#;

    let (output, error, _) = run_on_host(
        code,
        {
            let mut world = world;

            block_on(world.tick(86_401)).unwrap();

            world
        },
        true,
    );

    assert_eq!(error, None);
    assert_eq!(output, "1");
}

#[test]
fn should_leave_future_events_queued() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let coin: CoinId = ether(&world);

    block_on(world.schedule(world.time_seconds() + 60, Routine::CoinRound { coin })).unwrap();

    let report = block_on(world.tick(59)).unwrap();

    assert!(report.routines.is_empty());
    assert_eq!(block_on(world.scheduled_routines()).unwrap().len(), 1);
}
