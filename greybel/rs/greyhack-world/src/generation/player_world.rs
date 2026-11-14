use greyhack_behaviour::{DeviceKind, NetworkKind, main_router_address, position_for_ip};
use greyhack_world_creation::{RepositoryDraft, assign_player_libraries, kernel_router};

use super::{PlayerOptions, WorldOptions, preset};
use crate::{
    ComputerId, Database, DeviceLayout, Gender, LanPlace, LayoutFailure, NetworkDraft, Person,
    PersonId, Result, Rng, RouterDraft, Word, World, WorldError,
};

/// Creates the player world in `db` in one transaction: default repository and coins, player router
/// and computer, bank account, and mail.
pub(crate) async fn player_world(options: &WorldOptions, db: Database) -> Result<World> {
    let mut rng = Rng::new(options.seed);
    let mut player_rng = rng.fork("player");
    let mut repo_rng = rng.fork("apt");
    let mut chain_rng = rng.fork("blockchain");
    let player = &options.player;
    let time = greyhack_behaviour::parse_date(&player.date);
    let world = World::on(db, options.seed, ComputerId(0), time);

    world.handle.world_mut().rng = rng;
    world.begin().await?;

    let repository = RepositoryDraft::default_repository(&mut repo_rng)
        .store(&world.modules.libraries.generator)
        .await?;

    world
        .modules
        .services
        .apt
        .insert_repository(&repository)
        .await?;

    // New-game players have no currencies. Sandbox defaults apply only outside mirrored worlds.
    if player.network.is_none() {
        for draft in greyhack_world_economy::default_coins(&mut chain_rng, &player.date) {
            let coin = world
                .modules
                .economy
                .coins
                .insert_coin(&draft.coin, Some(&draft.password))
                .await?;

            for price in &draft.prices {
                world
                    .modules
                    .economy
                    .coins
                    .insert_price(coin, price)
                    .await?;
            }
        }
    }

    if let Some(bank_number) = &player.bank_number {
        world
            .modules
            .economy
            .bank
            .add_bank(bank_number, &player.password, player.bank_money)
            .await?;
    }

    if let Some(mail_address) = &player.mail_address {
        world
            .modules
            .npcs
            .mail
            .add_player_mail(
                mail_address,
                player.mail_password.as_deref().unwrap_or(&player.password),
            )
            .await?;
    }

    if let Some(layout) = &player.network {
        world.handle.world_mut().player_person = insert_player_person(&world, player).await?;

        let mut layout = layout.clone();

        for node in &mut layout.nodes {
            if let DeviceLayout::Computer(draft) = &mut node.device
                && draft.is_player
            {
                for user in &mut draft.users {
                    if user.name == player.user {
                        user.person = Some(world.handle.player_person());
                    }
                }
            }
        }

        let (_, computer) = world
            .modules
            .creation
            .network_generation
            .add_network_layout(&layout)
            .await?;

        world.handle.world_mut().player =
            computer.ok_or_else(|| WorldError::Layout(LayoutFailure::NoPlayerComputer))?;
        world.commit().await?;

        return Ok(world);
    }

    let network = world
        .modules
        .core
        .networks
        .insert_network(&NetworkDraft {
            kind: NetworkKind::Residential,
            has_plan: false,
            public_ip: player.public_ip.clone(),
            domain: String::new(),
            contact: None,
            position: position_for_ip(&player.public_ip),
        })
        .await?;
    let essid = Some(format!("{}_net", player.computer_name));
    let router_ip = main_router_address(0);
    let bssid = Some(player_rng.mac());
    let wifi_password = Word::Password.generate(&mut player_rng);
    let kernel = kernel_router(&mut player_rng);
    let root_password =
        Word::Password.generate(&mut Rng::root_password(options.seed, &player.public_ip));
    let computer = world.modules.creation.network_generation.router_computer(
        &player.public_ip,
        DeviceKind::Router,
        &router_ip,
        &root_password,
        *kernel,
    );
    let router = world
        .modules
        .creation
        .network_generation
        .insert_router(
            &RouterDraft {
                essid,
                bssid,
                wifi_password,
                firewall_rules: Vec::new(),
                firewall_active: true,
                computer,
            },
            network,
            None,
        )
        .await?;

    world.handle.world_mut().player_person = insert_player_person(&world, player).await?;

    let mut computer = preset::player_computer(player);

    assign_player_libraries(&mut computer, &mut player_rng);

    for user in &mut computer.users {
        if user.name == player.user {
            user.person = Some(world.handle.player_person());
        }
    }

    world.handle.world_mut().player = world
        .modules
        .libraries
        .libraries
        .insert_full_computer(
            &computer,
            network,
            &LanPlace::computer_under(router.computer(), router, &router_ip),
        )
        .await?;
    world.modules.libraries.check.ensure().await?;
    world.modules.economy.daily_upkeep.ensure().await?;
    world.commit().await?;

    Ok(world)
}

/// Inserts the player person and accounts from `player`.
async fn insert_player_person(world: &World, player: &PlayerOptions) -> Result<PersonId> {
    let person = world
        .modules
        .core
        .computers
        .insert_person(&Person {
            id: PersonId(0),
            name: player.user.clone(),
            surname: String::new(),
            gender: Gender::Male,
        })
        .await?;

    world
        .modules
        .npcs
        .mail
        .set_mail_address(person, player.mail_address.as_deref())
        .await?;
    world
        .modules
        .economy
        .bank
        .set_bank_number(person, player.bank_number.as_deref())
        .await?;

    Ok(person)
}
