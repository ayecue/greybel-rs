use greybel_test_support::future::block_on;
use greyhack_behaviour::{NetworkKind, SECONDS_PER_DAY, kind_for_ip};
use greyhack_world::{GuildDraft, NetworkOptions, Rng, World};

pub const SHARES: u32 = 100_000;

/// Adds a supermarket network and returns its public address.
pub fn add_company(world: &mut World) -> String {
    let mut rng = Rng::new(31);

    loop {
        let address = rng.public_ip();

        if kind_for_ip(world.world_seed(), &address) == NetworkKind::Supermarket
            && block_on(world.router_at(&address)).unwrap().is_none()
        {
            block_on(world.add_network(&NetworkOptions {
                public_ip: Some(address.clone()),
                ..NetworkOptions::default()
            }))
            .unwrap();

            return address;
        }
    }
}

/// Makes guild `Owls` hold the Crown by letting it own the company at `address` for fifteen days.
pub fn crown(world: &mut World, address: &str) {
    block_on(world.create_guild(&GuildDraft {
        name: "Owls".to_owned(),
        bank_account: "owls-bank".to_owned(),
        total_exp: 275 * 25,
        investment_perks: 1,
        admin_restore_perks: 0,
        player_member: true,
    }))
    .unwrap()
    .unwrap();
    assert!(block_on(world.set_company_shares(address, "owls-bank", SHARES)).unwrap());
    block_on(world.tick(0)).unwrap();
    block_on(world.tick(15 * SECONDS_PER_DAY)).unwrap();
}
