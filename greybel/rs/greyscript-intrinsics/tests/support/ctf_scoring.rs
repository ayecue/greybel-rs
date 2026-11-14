use greybel_test_support::future::block_on;
use greyhack_world::{ComputerOptions, CtfEvent, PlayerOptions, World};

use crate::support::read_computer;

/// A world whose player can score the CTF event "mission" of another computer, and that computer.
pub fn scoring_world() -> (World, greyhack_world::ComputerId) {
    let world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let router = read_computer(&world, world.handle.player()).router.unwrap();
    let owner = block_on(world.modules.creation.network_generation.add_computer_to(
        router,
        &ComputerOptions {
            lan_ip: Some("192.168.0.60".to_owned()),
            ..ComputerOptions::default()
        },
    ))
    .unwrap()
    .expect("the event owner is created");

    block_on(
        world
            .modules
            .player
            .ctf
            .create_ctf_account(owner, "creator", "old"),
    )
    .expect("the creator CTF account is created");

    let creator = block_on(world.modules.npcs.npcs.person_on(owner))
        .expect("the world reads")
        .expect("the event creator");

    block_on(world.modules.economy.bank.add_bank("owner", "owner", 0.0))
        .expect("the creator bank is added");
    block_on(
        world
            .modules
            .economy
            .bank
            .set_bank_number(creator, Some("owner")),
    )
    .expect("the creator bank is linked");
    block_on(world.modules.player.ctf.create_ctf_event(&CtfEvent {
        name: "mission".to_owned(),
        user: "ctfuser".to_owned(),
        password_hash: greyhack_behaviour::hash_password("old"),
        description: "desc".to_owned(),
        public_address: "1.2.3.4".to_owned(),
        creator,
        creator_name: "creator".to_owned(),
        mail_content: "mail".to_owned(),
        creator_bank_user: "owner".to_owned(),
        quality_rank: 3,
        difficulty_rank: 3,
        num_votes: 1,
        is_rated: false,
    }))
    .expect("the CTF event is created");
    assert!(
        block_on(
            world
                .modules
                .player
                .ctf
                .add_ctf_player("mission", world.handle.player())
        )
        .unwrap()
    );
    assert!(
        block_on(
            world
                .modules
                .player
                .ctf
                .player_success("mission", world.handle.player())
        )
        .unwrap()
    );

    (world, owner)
}
