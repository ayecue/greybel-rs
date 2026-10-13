use greyhack_world::{ComputerOptions, MiningHardware, PlayerOptions, World};

use crate::support::{computer, save_copy, subwallet};

#[tokio::test]
async fn should_choose_a_miner_with_power_and_keep_other_computers_pending() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let router = computer(&world, world.player()).await.router.unwrap();
    let weak = world
        .add_computer_to(
            router,
            &ComputerOptions {
                mining_hardware: MiningHardware {
                    cpu_speed: 0.0,
                    cpu_cores: 0,
                    ram_model: 0,
                    gpu_hashrate: 0.0,
                },
                ..ComputerOptions::default()
            },
        )
        .await
        .unwrap()
        .unwrap();
    let powerful = world
        .add_computer_to(router, &ComputerOptions::default())
        .await
        .unwrap()
        .unwrap();
    let coin = world.coin_by_name("Ether").await.unwrap().unwrap();
    let wallet = world
        .add_wallet("alice", "pw", "pin", "test")
        .await
        .unwrap();

    let mut account = subwallet(coin.id, "alice", 0.0);

    account.password_hash = greyhack_behaviour::hash_password("");
    world.add_subwallet(&account, "").await.unwrap();
    world
        .schedule_mine_on(coin.id, "alice", weak)
        .await
        .unwrap()
        .unwrap();
    world
        .schedule_mine_on(coin.id, "alice", powerful)
        .await
        .unwrap()
        .unwrap();

    world.tick(0).await.unwrap();

    assert_eq!(
        world
            .subwallet(coin.id, "alice")
            .await
            .unwrap()
            .unwrap()
            .balance,
        coin.reward
    );
    assert_eq!(world.count_active_nodes(wallet, coin.id).await.unwrap(), 1);
    world.tick(60).await.unwrap();
    assert_eq!(world.count_active_nodes(wallet, coin.id).await.unwrap(), 1);
    assert_eq!(
        world
            .subwallet(coin.id, "alice")
            .await
            .unwrap()
            .unwrap()
            .balance,
        coin.reward
    );
}

#[tokio::test]
async fn should_compute_the_csharp_hardware_weight() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let router = computer(&world, world.player()).await.router.unwrap();
    let machine = world
        .add_computer_to(
            router,
            &ComputerOptions {
                mining_hardware: MiningHardware {
                    cpu_speed: 2.0,
                    cpu_cores: 4,
                    ram_model: 3,
                    gpu_hashrate: 5.0,
                },
                ..ComputerOptions::default()
            },
        )
        .await
        .unwrap()
        .unwrap();

    assert_eq!(
        world.hardware(machine).await.unwrap().mining_power(),
        1.2 + 3.072 + 5.0
    );
}

#[tokio::test]
async fn should_preserve_mining_hardware() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let router = computer(&world, world.player()).await.router.unwrap();
    let machine = world
        .add_computer_to(
            router,
            &ComputerOptions {
                mining_hardware: MiningHardware {
                    cpu_speed: 2.5,
                    cpu_cores: 8,
                    ram_model: 4,
                    gpu_hashrate: 12.0,
                },
                ..ComputerOptions::default()
            },
        )
        .await
        .unwrap()
        .unwrap();

    let database = save_copy(&mut world).await;
    let restored = World::load(&database).await.unwrap();
    let hardware = restored.hardware(machine).await.unwrap();

    assert_eq!(hardware.cpus[0].speed, 2.5);
    assert_eq!(hardware.cpus[0].cores, 8);
    assert_eq!(hardware.rams[0].model, 4);
    assert_eq!(hardware.gpus[0].hashrate, 12.0);
}
