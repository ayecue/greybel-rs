use greybel_test_support::future::block_on;
use greyhack_behaviour::{NetworkKind, Rng, SECONDS_PER_DAY, kind_for_ip};
use greyhack_world::World;
use greyhack_world_creation::NetworkOptions;
use greyhack_world_guilds::GuildDraft;

pub const SHARES: u32 = 100_000;

/// Adds a supermarket network and returns its public address.
pub fn add_company(world: &mut World) -> String {
    let mut rng = Rng::new(31);

    loop {
        let address = rng.public_ip();

        if kind_for_ip(&world.handle.world_seed(), &address) == NetworkKind::Supermarket
            && block_on(world.modules.core.networks.routers.router_at(&address))
                .unwrap()
                .is_none()
        {
            block_on(
                world
                    .modules
                    .creation
                    .network_generation
                    .add_network(&NetworkOptions {
                        public_ip: Some(address.clone()),
                        ..NetworkOptions::default()
                    }),
            )
            .unwrap();

            return address;
        }
    }
}

/// Makes guild `Owls` hold the Crown by letting it own the company at `address` for fifteen days.
pub fn crown(world: &mut World, address: &str) {
    block_on(world.modules.guilds.guilds.create_guild(&GuildDraft {
        name: "Owls".to_owned(),
        bank_account: "owls-bank".to_owned(),
        total_exp: 275 * 25,
        investment_perks: 1,
        admin_restore_perks: 0,
        player_member: true,
    }))
    .unwrap()
    .unwrap();
    assert!(
        block_on(
            world
                .modules
                .guilds
                .guilds
                .set_company_shares(address, "owls-bank", SHARES)
        )
        .unwrap()
    );
    block_on(world.tick(0)).unwrap();
    block_on(world.tick(15 * SECONDS_PER_DAY)).unwrap();
}
