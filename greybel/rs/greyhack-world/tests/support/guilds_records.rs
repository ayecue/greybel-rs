use greyhack_behaviour::{NetworkKind, ROOT_USER, Reward, Rng, SECONDS_PER_DAY, kind_for_ip};
use greyhack_world::{PlayerOptions, World};
use greyhack_world_core::{ComputerId, NetworkId};
use greyhack_world_creation::NetworkOptions;
use greyhack_world_guilds::GuildDraft;
use greyhack_world_libraries::{Library, MemoryZone, Weakness};

/// Experience of a level-5 guild, which may invest in one company.
pub(crate) const LEVEL_FIVE: u32 = 275 * 25;
/// Shares a supermarket company issues.
pub(crate) const SHARES: u32 = 100_000;
pub(crate) const DAY: u64 = SECONDS_PER_DAY;

/// A world with `count` supermarket networks, and their public addresses.
pub async fn world_with_companies(count: usize) -> (World, Vec<String>) {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let mut rng = Rng::new(11);
    let mut addresses = Vec::new();

    while addresses.len() < count {
        let address = rng.public_ip();

        if kind_for_ip(&world.handle.world().world_seed, &address) != NetworkKind::Supermarket
            || world
                .modules
                .core
                .networks
                .routers
                .router_at(&address)
                .await
                .unwrap()
                .is_some()
        {
            continue;
        }

        world
            .modules
            .creation
            .network_generation
            .add_network(&NetworkOptions {
                public_ip: Some(address.clone()),
                ..NetworkOptions::default()
            })
            .await
            .unwrap();
        addresses.push(address);
    }

    (world, addresses)
}

/// Creates a guild that may invest in one company and holds its shares under `account`.
pub async fn guild(world: &mut World, name: &str, account: &str, join: bool) {
    world
        .modules
        .guilds
        .guilds
        .create_guild(&GuildDraft {
            name: name.to_owned(),
            bank_account: account.to_owned(),
            total_exp: LEVEL_FIVE,
            investment_perks: 1,
            admin_restore_perks: 0,
            player_member: join,
            player_owner: join,
        })
        .await
        .unwrap()
        .unwrap();
}

/// Lets `account` own all of the company at `address`.
pub async fn own(world: &mut World, address: &str, account: &str) {
    assert!(
        world
            .modules
            .guilds
            .guilds
            .set_company_shares(address, account, SHARES)
            .await
            .unwrap()
    );
}

/// Whether the guild named `name` holds the Crown.
pub async fn has_crown(world: &World, name: &str) -> bool {
    world
        .modules
        .guilds
        .guilds
        .guilds()
        .await
        .unwrap()
        .into_iter()
        .find(|guild| guild.name == name)
        .expect("the guild")
        .has_crown
}

/// Gives guild `name` the Crown by letting it lead the ranking for the days the game asks.
pub async fn crown(world: &mut World, name: &str, account: &str, address: &str) {
    guild(world, name, account, true).await;
    own(world, address, account).await;
    world.tick(0).await.unwrap();
    world.tick(15 * DAY).await.unwrap();

    assert!(has_crown(world, name).await);
}

pub async fn network_of(world: &World, address: &str) -> NetworkId {
    world
        .modules
        .core
        .networks
        .routers
        .router_at(address)
        .await
        .unwrap()
        .unwrap()
        .network
}

pub(crate) const LIBRARY_PATH: &str = "/lib/libssh.so";
pub(crate) const ADDRESS: &str = "0x1234";
pub(crate) const UNSEC_VALUE: &str = "buffer";

pub async fn install(world: &World, computer: ComputerId) {
    let file = world
        .modules
        .core
        .files
        .lookup
        .file(computer, LIBRARY_PATH, false)
        .await
        .unwrap()
        .expect("a library");

    world
        .modules
        .libraries
        .generator
        .set_full_library_of(
            &file,
            Some(&Library {
                kind: greyhack_behaviour::LibraryKind::Ssh,
                version: "9.9.9".to_owned(),
                zones: vec![MemoryZone {
                    address: ADDRESS.to_owned(),
                    hide: false,
                    weaknesses: vec![Weakness {
                        remote: true,
                        unsec_value: UNSEC_VALUE.to_owned(),
                        details: "Test weakness".to_owned(),
                        requirements: Vec::new(),
                        reward: Reward::Shell,
                        user: ROOT_USER.to_owned(),
                        required_lib: None,
                        req_lib_version: "1.0.0".to_owned(),
                        metaxploit_version: "1.0.0".to_owned(),
                        num_register_users: 0,
                        num_port_forward: 0,
                        num_conn_gateway: 0,
                        path_exist: String::new(),
                        random_path: "/root".to_owned(),
                    }],
                }],
            }),
        )
        .await
        .unwrap();
}
