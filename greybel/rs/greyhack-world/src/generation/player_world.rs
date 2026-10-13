use greyhack_behaviour::{DeviceKind, NetworkKind, main_router_address, position_for_ip};

use super::{PlayerOptions, WorldOptions, blockchain, computer, kernel_router, preset};
use crate::{
    ComputerId, Database, DeviceLayout, Gender, LanPlace, LayoutFailure, NetworkDraft, Person,
    PersonId, Repository, Result, Rng, RouterDraft, Word, World, WorldError,
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
    let mut world = World::on(db, options.seed, ComputerId(0), time);

    world.rng = rng;
    world.begin().await?;
    world
        .db
        .insert_repository(&Repository::default_repository(&mut repo_rng))
        .await?;

    // New-game players have no currencies. Sandbox defaults apply only outside mirrored worlds.
    if player.network.is_none() {
        for draft in blockchain::default_coins(&mut chain_rng, &player.date) {
            let coin = world
                .db
                .insert_coin(&draft.coin, Some(&draft.password))
                .await?;

            for price in &draft.prices {
                world.db.insert_price(coin, price).await?;
            }
        }
    }

    if let Some(bank_number) = &player.bank_number {
        world
            .add_bank(bank_number, &player.password, player.bank_money)
            .await?;
    }

    if let Some(mail_address) = &player.mail_address {
        world
            .add_player_mail(
                mail_address,
                player.mail_password.as_deref().unwrap_or(&player.password),
            )
            .await?;
    }

    if let Some(layout) = &player.network {
        world.player_person = insert_player_person(&world, player).await?;

        let mut layout = layout.clone();

        for node in &mut layout.nodes {
            if let DeviceLayout::Computer(draft) = &mut node.device
                && draft.is_player
            {
                for user in &mut draft.users {
                    if user.name == player.user {
                        user.person = Some(world.player_person);
                    }
                }
            }
        }

        let (_, computer) = world.add_network_layout(&layout).await?;

        world.player =
            computer.ok_or_else(|| WorldError::Layout(LayoutFailure::NoPlayerComputer))?;
        world.commit().await?;

        return Ok(world);
    }

    let network = world
        .db
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
    let computer = world.router_computer(
        &player.public_ip,
        DeviceKind::Router,
        &router_ip,
        &root_password,
        *kernel,
    );
    let router = world
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

    world.player_person = insert_player_person(&world, player).await?;

    let mut computer = preset::player_computer(player);

    computer::assign_player_libraries(&mut computer, &mut player_rng);

    for user in &mut computer.users {
        if user.name == player.user {
            user.person = Some(world.player_person);
        }
    }

    world.player = world
        .db
        .insert_computer(
            &computer,
            network,
            &LanPlace::computer_under(router.computer(), router, &router_ip),
        )
        .await?;
    world.commit().await?;

    Ok(world)
}

/// Inserts the player person and accounts from `player`.
async fn insert_player_person(world: &World, player: &PlayerOptions) -> Result<PersonId> {
    world
        .db
        .insert_person(&Person {
            id: PersonId(0),
            name: player.user.clone(),
            surname: String::new(),
            gender: Gender::Male,
            mail_address: player.mail_address.clone(),
            bank_number: player.bank_number.clone(),
        })
        .await
}
