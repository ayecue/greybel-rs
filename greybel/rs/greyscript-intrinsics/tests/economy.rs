use std::rc::Rc;

use duplex_test_support::local::run_local;
use greybel_test_support::future::block_on;
use greybel_test_support::greyhack;
use greybel_test_support::interpreter::snapshot_options;
use greyhack_behaviour::ServiceId;
use greyhack_world::{DatabaseOpen, PlayerOptions, World};
use greyhack_world_core::Database;
use greyhack_world_economy::TransferFailure;
use greyhack_world_operations::WorldServer;
use scheduler::ManualClock;

use crate::support::economy::{
    add_subwallet, ether, minutes, subwallet, world_with_ether_created_by_test,
};
use crate::support::scripts::{run_code_in, run_on_host};

#[test]
fn should_transfer_balance_and_record_both_transaction_sides() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let coin = ether(&world);

    add_subwallet(&mut world, &subwallet(coin, "alice", 10.0));
    add_subwallet(&mut world, &subwallet(coin, "bob", 0.0));

    let result = block_on(world.modules.economy.wallets.transaction(
        coin,
        "alice",
        "bob",
        3,
        "01/Jan/2000 - 00:00",
    ))
    .unwrap();

    assert_eq!(result, Ok(()));
    assert_eq!(
        block_on(world.modules.economy.wallets.subwallet(coin, "alice"))
            .unwrap()
            .unwrap()
            .balance,
        7.0
    );
    assert_eq!(
        block_on(world.modules.economy.wallets.subwallet(coin, "bob"))
            .unwrap()
            .unwrap()
            .balance,
        3.0
    );
    assert_eq!(
        block_on(
            world
                .modules
                .economy
                .wallets
                .last_transaction(coin, "alice")
        )
        .unwrap()
        .unwrap()
        .direction,
        1
    );
    assert_eq!(
        block_on(world.modules.economy.wallets.last_transaction(coin, "bob"))
            .unwrap()
            .unwrap()
            .direction,
        0
    );
}

#[test]
fn should_reject_a_transaction_with_insufficient_funds() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let coin = ether(&world);

    add_subwallet(&mut world, &subwallet(coin, "alice", 1.0));
    add_subwallet(&mut world, &subwallet(coin, "bob", 0.0));

    let result = block_on(
        world
            .modules
            .economy
            .wallets
            .transaction(coin, "alice", "bob", 2, "date"),
    )
    .unwrap();

    assert_eq!(result, Err(TransferFailure::InsufficientCoins));
}

#[test]
fn should_mine_into_the_subwallet_and_stop_at_the_supply_limit() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let coin = ether(&world);
    let mut target = block_on(world.modules.economy.coins.coin(coin))
        .unwrap()
        .unwrap();

    add_subwallet(&mut world, &subwallet(coin, "alice", 0.0));
    target.max_supply = target.mined_coins + target.reward;
    block_on(world.modules.economy.coins.save_coin(&target)).unwrap();

    let first = block_on(world.modules.economy.coins.mine(coin, "alice")).unwrap();
    let second = block_on(world.modules.economy.coins.mine(coin, "alice")).unwrap();

    assert_eq!(first, Ok(()));
    assert!(second.is_err());
    assert_eq!(
        block_on(world.modules.economy.wallets.subwallet(coin, "alice"))
            .unwrap()
            .unwrap()
            .balance,
        1.0
    );
}

#[test]
fn should_delete_a_subwallet_and_reduce_the_mined_total() {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let coin = ether(&world);
    let before = block_on(world.modules.economy.coins.coin(coin))
        .unwrap()
        .unwrap()
        .mined_coins;

    add_subwallet(&mut world, &subwallet(coin, "alice", 4.0));

    let deleted = block_on(
        world
            .modules
            .economy
            .mine_jobs
            .delete_subwallet(coin, "alice"),
    )
    .unwrap();

    assert!(deleted);
    assert!(
        block_on(world.modules.economy.wallets.subwallet(coin, "alice"))
            .unwrap()
            .is_none()
    );
    assert_eq!(
        block_on(world.modules.economy.coins.coin(coin))
            .unwrap()
            .unwrap()
            .mined_coins,
        before - 4.0
    );
}

#[test]
fn should_keep_other_coin_handles_stable_after_deleting_a_coin() {
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let ether = ether(&world);
    let bitcoin = block_on(world.modules.economy.coins.coin_by_name("Bitcoin"))
        .unwrap()
        .unwrap()
        .id;
    let deleted = block_on(world.modules.economy.mine_jobs.delete_coin("Ether")).unwrap();

    assert!(deleted);
    assert!(
        block_on(world.modules.economy.coins.coin(ether))
            .unwrap()
            .is_none()
    );
    assert_eq!(
        block_on(world.modules.economy.coins.coin(bitcoin))
            .unwrap()
            .unwrap()
            .name,
        "Bitcoin"
    );
}

#[test]
fn should_list_global_coins_and_report_no_active_mining_nodes() {
    let code = r#"
chain = include_lib("/lib/blockchain.so")
wallet = chain.create_wallet("test", "test")
print wallet.list_global_coins.indexOf("Ether") != null
print wallet.show_nodes("Ether")
print wallet.show_nodes("missing")
"#;
    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(output, "1\n0\nshow_nodes: coin does not exist");
}

#[test]
fn should_reset_a_wallet_password_and_log_in_with_the_new_one() {
    let code = r#"
chain = include_lib("/lib/blockchain.so")
wallet = chain.create_wallet("test", "test")
get_greybel_util().tick(86401)
print wallet.reset_password("newpass")
print chain.login_wallet("test", "newpass") != null
"#;
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");

    block_on(world.tick(86_401)).unwrap();
    // Keep the unrelated empty-queue next_due_at conversion failure out of this native API test.
    block_on(world.modules.services.restart.schedule(
        world.handle.time_seconds() + 86_402,
        world.handle.player(),
        ServiceId::Ssh,
    ))
    .unwrap();

    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "1\n1");
}

#[test]
fn should_mine_and_delete_a_subwallet_through_the_native_api() {
    let world = world_with_ether_created_by_test();
    let code = r#"
chain = include_lib("/lib/blockchain.so")
wallet = chain.create_wallet("test", "test")
coin = chain.get_coin("Ether", "test", "test")
print coin.create_subwallet("test", wallet.get_pin, "alice", "pw")
sub = coin.get_subwallet("alice")
print sub.mining
print sub.get_balance
print sub.delete
print coin.get_subwallet("alice")
"#;

    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "1\n1\n1\n1\nget_subwallet: The account does not exist"
    );
}

#[test]
fn should_return_the_players_coin_name() {
    let world = world_with_ether_created_by_test();

    let (output, error, _) = run_on_host(
        "print include_lib(\"/lib/blockchain.so\").get_coin_name(\"test\", \"test\")",
        world,
        true,
    );

    assert_eq!(error, None);
    assert_eq!(output, "Ether");
}

#[test]
fn should_delete_a_coin_through_the_native_api() {
    let world = world_with_ether_created_by_test();
    let code =
        "print include_lib(\"/lib/blockchain.so\").delete_coin(\"Ether\", \"test\", \"test\")";

    let (output, error, world) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "1");
    assert!(
        block_on(world.modules.economy.coins.coin_by_name("Ether"))
            .unwrap()
            .is_none()
    );
}

/// `mining` responds once its job wins one of the coin's rounds: with the round due in an hour of world time, the
/// program waits while the world ticks there.
#[test]
fn should_respond_to_mining_once_its_job_wins_a_round() {
    let world = world_with_ether_created_by_test();
    let coin = ether(&world);
    let start = world.date();
    let now = world.handle.time_seconds();
    let mut target = block_on(world.modules.economy.coins.coin(coin))
        .unwrap()
        .unwrap();

    target.cycle_hours = 1.0;
    target.last_cycle_at = Some(now);
    block_on(world.modules.economy.coins.save_coin(&target)).unwrap();

    let code = r#"
chain = include_lib("/lib/blockchain.so")
wallet = chain.create_wallet("test", "test")
coin = chain.get_coin("Ether", "test", "test")
coin.create_subwallet("test", wallet.get_pin, "alice", "pw")
sub = coin.get_subwallet("alice")
print sub.mining
print sub.get_balance
print current_date
"#;
    let printed = run_local(async {
        let server = WorldServer::with_database(
            world,
            Database::in_memory().await.expect("an in-memory database"),
            Rc::new(ManualClock::advance_on_sleep()),
        )
        .await
        .expect("the world is written to the database");
        let (stdout, options) = snapshot_options();
        let error =
            greyhack::run_in(&server, code, &PlayerOptions::default().user, true, options).await;

        assert_eq!(error, None);

        stdout.text()
    });
    let lines: Vec<&str> = printed.lines().collect();

    assert_eq!(lines[..2], ["1", "1"]);
    assert!(
        minutes(lines[2]) >= minutes(&start) + 60,
        "{} is an hour after {start}",
        lines[2]
    );
}

/// `set_address` answers the game's text for an address with no network, which the native returns.
#[test]
fn should_answer_the_games_text_for_a_coin_address_without_a_network() {
    let world = world_with_ether_created_by_test();
    let code = r#"
chain = include_lib("/lib/blockchain.so")
wallet = chain.create_wallet("test", "test")
coin = chain.get_coin("Ether", "test", "test")
print coin.set_address("nowhere.test")
print coin.set_address("1.2.3.4")
print coin.get_address
"#;

    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "set_address: address nowhere.testnot found\n1\n1.2.3.4"
    );
}

/// A coin's reward is 1 to 100,000,000 whole coins and its mining cycle 1 to 2160 whole hours
/// (`CoinIntrinsics.set_reward`, `set_cycle_mining`).
#[test]
fn should_keep_a_coins_reward_and_cycle_within_the_games_limits() {
    let world = world_with_ether_created_by_test();
    let code = r#"
chain = include_lib("/lib/blockchain.so")
wallet = chain.create_wallet("test", "test")
coin = chain.get_coin("Ether", "test", "test")
print coin.set_reward(100000001)
print coin.set_reward(0.5)
print coin.set_reward(7.9)
print coin.get_reward
print coin.set_cycle_mining(2161)
print coin.set_cycle_mining(0)
print coin.set_cycle_mining(3.5)
print coin.get_cycle_mining
"#;

    let (output, error, _) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(
        output,
        "Error: The reward cannot exceed 100000000 coins\nError: The reward cannot be less than 1 coin\n1\n7\n\
         the mining cycle cannot exceed the maximum of 2160 hours\nthe mining cycle cannot be less than 1 hour\n1\n3"
    );
}
