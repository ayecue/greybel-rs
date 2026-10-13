use greybel_test_support::future::block_on;
use greyhack_world::{ComputerOptions, CtfEvent, PlayerOptions, World};

use crate::support::read_computer;

/// A world whose player can score the CTF event "mission" of another computer, and that computer.
pub fn scoring_world() -> (World, greyhack_world::ComputerId) {
    let mut world = block_on(World::new(&PlayerOptions::default())).expect("a world");
    let router = read_computer(&world, world.player()).router.unwrap();
    let owner = block_on(world.add_computer_to(
        router,
        &ComputerOptions {
            lan_ip: Some("192.168.0.60".to_owned()),
            ..ComputerOptions::default()
        },
    ))
    .unwrap()
    .expect("the event owner is created");

    block_on(world.create_ctf_account(owner, "creator", "old"))
        .expect("the creator CTF account is created");

    let creator = block_on(world.person_on(owner))
        .expect("the world reads")
        .expect("the event creator");
    let mut person = block_on(world.person(creator))
        .expect("the world reads")
        .expect("the event creator");

    person.bank_number = Some("owner".to_owned());
    block_on(world.add_bank("owner", "owner", 0.0)).expect("the creator bank is added");
    block_on(world.save_person(&person)).expect("the creator is saved");
    block_on(world.create_ctf_event(&CtfEvent {
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
    assert!(block_on(world.add_ctf_player("mission", world.player())).unwrap());
    assert!(block_on(world.player_success("mission", world.player())).unwrap());

    (world, owner)
}
