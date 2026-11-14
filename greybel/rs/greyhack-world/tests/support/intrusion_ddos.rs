use greyhack_behaviour::NetworkKind;
use greyhack_world::{GuildDraft, NetworkOptions, PlayerOptions, RouterId, World};

const SHARES: u32 = 100_000;
const GUILD_EXP: u32 = 275 * 25;

/// A world with a supermarket company; returns its public address and router.
pub async fn company() -> (World, String, RouterId) {
    let world = World::new(&PlayerOptions::default()).await.unwrap();
    let router = world
        .modules
        .creation
        .network_generation
        .add_network(&NetworkOptions::default())
        .await
        .unwrap();
    let row = world
        .modules
        .core
        .networks
        .routers
        .router(router)
        .await
        .unwrap()
        .unwrap();

    world
        .modules
        .core
        .networks
        .settings
        .set_network_kind(row.network, NetworkKind::Supermarket)
        .await
        .unwrap();

    (world, row.public_ip, router)
}

pub async fn guild_owning(world: &mut World, address: &str) {
    world
        .modules
        .guilds
        .guilds
        .create_guild(&GuildDraft {
            name: "Owls".to_owned(),
            bank_account: "owls-bank".to_owned(),
            total_exp: GUILD_EXP,
            investment_perks: 1,
            admin_restore_perks: 0,
            player_member: false,
        })
        .await
        .unwrap()
        .unwrap();
    world
        .modules
        .guilds
        .guilds
        .set_company_shares(address, "owls-bank", SHARES)
        .await
        .unwrap();
}
