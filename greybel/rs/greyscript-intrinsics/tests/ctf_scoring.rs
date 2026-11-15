use greybel_test_support::future::block_on;
use greyhack_world::World;
use greyhack_world_player::CtfFailure;

use crate::support::ctf_scoring::scoring_world;
use crate::support::save_copy;
use crate::support::scripts::run_on_host;

#[test]
fn should_credit_ctf_participant_and_creator_rewards() {
    let (world, owner) = scoring_world();
    let code = r#"
admin = include_lib("admin.so")
rewards = admin.rate_ctf_event("mission", 5, 4)
print rewards[0]
print rewards[1]
print admin.rate_ctf_event("mission", 5, 4)
"#;

    let (output, error, restored) = run_on_host(code, world, true);

    assert_eq!(error, None);
    assert_eq!(output, "675\n160\nYou have already rated this mission");
    assert_eq!(
        block_on(restored.modules.economy.bank.bank_account("test"))
            .unwrap()
            .unwrap()
            .money,
        3675.0
    );
    assert_eq!(
        block_on(restored.modules.economy.bank.bank_account("owner"))
            .unwrap()
            .unwrap()
            .money,
        160.0
    );

    let event = block_on(restored.modules.player.ctf.ctf_event("mission"))
        .unwrap()
        .unwrap();
    let players = block_on(restored.modules.player.ctf.ctf_players("mission")).unwrap();

    assert_eq!(event.quality_rank, 5);
    assert_eq!(event.difficulty_rank, 4);
    assert_eq!(event.num_votes, 1);
    assert!(event.is_rated);
    assert_eq!(players[0].quality_rate, 5);
    assert_eq!(players[0].difficulty_rate, 4);
    assert_eq!(
        block_on(restored.modules.player.ctf.ctf_creator_score(owner)).unwrap(),
        4.0
    );
}

#[tokio::test]
async fn should_preserve_ctf_ratings() {
    let (mut world, _) = scoring_world();

    world
        .modules
        .player
        .ctf
        .add_ctf_score("mission", world.handle.player(), 5, 4)
        .await
        .unwrap()
        .unwrap();

    let database = save_copy(&mut world).await;

    let restored = World::load(&database).await.unwrap();

    let event = restored
        .modules
        .player
        .ctf
        .ctf_event("mission")
        .await
        .unwrap()
        .unwrap();
    let players = restored
        .modules
        .player
        .ctf
        .ctf_players("mission")
        .await
        .unwrap();

    assert!(event.is_rated);
    assert_eq!(event.quality_rank, 5);
    assert_eq!(event.difficulty_rank, 4);
    assert_eq!(event.num_votes, 1);
    assert_eq!(players[0].quality_rate, 5);
    assert_eq!(players[0].difficulty_rate, 4);
    assert_eq!(event.creator_bank_user, "owner");
    assert_eq!(
        restored
            .modules
            .economy
            .bank
            .bank_account("test")
            .await
            .unwrap()
            .unwrap()
            .money,
        3675.0
    );
}

#[test]
fn should_reject_invalid_and_incomplete_ctf_ratings() {
    let (world, owner) = scoring_world();

    assert_eq!(
        block_on(
            world
                .modules
                .player
                .ctf
                .add_ctf_score("mission", world.handle.player(), 0, 4)
        )
        .unwrap(),
        Err(CtfFailure::InvalidScore)
    );
    assert_eq!(
        block_on(
            world
                .modules
                .player
                .ctf
                .add_ctf_score("mission", owner, 5, 4)
        )
        .unwrap(),
        Err(CtfFailure::Incomplete)
    );
}

/// A creator cannot rate their own event (`CTF_Event.AddScore`), and the refusal neither changes
/// its ratings nor removes their completed participation.
#[test]
fn should_refuse_a_creators_own_ctf_rating_and_keep_them_in_its_players() {
    let (world, owner) = scoring_world();

    assert!(block_on(world.modules.player.ctf.add_ctf_player("mission", owner)).unwrap());
    assert!(block_on(world.modules.player.ctf.player_success("mission", owner)).unwrap());
    assert_eq!(
        block_on(
            world
                .modules
                .player
                .ctf
                .add_ctf_score("mission", owner, 5, 4)
        )
        .unwrap(),
        Err(CtfFailure::OwnEvent)
    );

    let event = block_on(world.modules.player.ctf.ctf_event("mission"))
        .unwrap()
        .unwrap();

    assert!(!event.is_rated);
    assert_eq!(
        (event.quality_rank, event.difficulty_rank, event.num_votes),
        (3, 3, 1)
    );

    let creator = block_on(world.modules.npcs.npcs.person_on(owner))
        .unwrap()
        .unwrap();
    let players = block_on(world.modules.player.ctf.ctf_players("mission")).unwrap();

    assert!(players.iter().any(|player| player.person == creator));
    assert_eq!(
        block_on(world.modules.economy.bank.bank_account("owner"))
            .unwrap()
            .unwrap()
            .money,
        0.0
    );
    assert_eq!(
        block_on(
            world
                .modules
                .player
                .ctf
                .add_ctf_score("mission", owner, 5, 4)
        )
        .unwrap(),
        Err(CtfFailure::OwnEvent)
    );
}
