use greyhack_behaviour::{NetworkKind, ROOT_USER, Reward, SECONDS_PER_DAY, kind_for_ip};

use crate::{
    ComputerId, GuildDraft, Library, MemoryZone, NetworkId, NetworkOptions, PlayerOptions, Rng,
    Weakness, World,
};

/// Experience of a level-5 guild, which may invest in one company.
pub(in crate::systems) const LEVEL_FIVE: u32 = 275 * 25;
/// Shares a supermarket company issues.
pub(in crate::systems) const SHARES: u32 = 100_000;
pub(in crate::systems) const DAY: u64 = SECONDS_PER_DAY;

/// A world with `count` supermarket networks, and their public addresses.
pub(in crate::systems) async fn world_with_companies(count: usize) -> (World, Vec<String>) {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let mut rng = Rng::new(11);
    let mut addresses = Vec::new();

    while addresses.len() < count {
        let address = rng.public_ip();

        if kind_for_ip(&world.world_seed, &address) != NetworkKind::Supermarket
            || world.router_at(&address).await.unwrap().is_some()
        {
            continue;
        }

        world
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
pub(in crate::systems) async fn guild(world: &mut World, name: &str, account: &str, join: bool) {
    world
        .create_guild(&GuildDraft {
            name: name.to_owned(),
            bank_account: account.to_owned(),
            total_exp: LEVEL_FIVE,
            investment_perks: 1,
            admin_restore_perks: 0,
            player_member: join,
        })
        .await
        .unwrap()
        .unwrap();
}

/// Lets `account` own all of the company at `address`.
pub(in crate::systems) async fn own(world: &mut World, address: &str, account: &str) {
    assert!(
        world
            .set_company_shares(address, account, SHARES)
            .await
            .unwrap()
    );
}

/// Whether the guild named `name` holds the Crown.
pub(in crate::systems) async fn has_crown(world: &World, name: &str) -> bool {
    world
        .guilds()
        .await
        .unwrap()
        .into_iter()
        .find(|guild| guild.name == name)
        .expect("the guild")
        .has_crown
}

/// Gives guild `name` the Crown by letting it lead the ranking for the days the game asks.
pub(in crate::systems) async fn crown(world: &mut World, name: &str, account: &str, address: &str) {
    guild(world, name, account, true).await;
    own(world, address, account).await;
    world.tick(0).await.unwrap();
    world.tick(15 * DAY).await.unwrap();

    assert!(has_crown(world, name).await);
}

pub(in crate::systems) async fn network_of(world: &World, address: &str) -> NetworkId {
    world.router_at(address).await.unwrap().unwrap().network
}

pub(in crate::systems) const LIBRARY_PATH: &str = "/lib/libssh.so";
pub(in crate::systems) const ADDRESS: &str = "0x1234";
pub(in crate::systems) const UNSEC_VALUE: &str = "buffer";

pub(in crate::systems) async fn install(world: &World, computer: ComputerId) {
    let file = world
        .file(computer, LIBRARY_PATH, false)
        .await
        .unwrap()
        .expect("a library");

    world
        .set_library_of(
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
