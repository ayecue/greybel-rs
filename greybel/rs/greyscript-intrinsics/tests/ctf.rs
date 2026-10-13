use greyhack_behaviour::hash_password;
use greyhack_world::{CtfEvent, PlayerOptions, World};

use crate::support::save_copy;
use crate::support::scripts::run_code_in;

#[test]
fn should_read_ctf_event_data_and_complete_a_player_once() {
    let code = r#"
admin = include_lib("admin.so")
admin.create_ctf_account("ctfuser", "old")
admin.create_ctf_event({"eventName": "mission", "user": "ctfuser", "password": "old", "description": "desc", "publicAddress": "1.2.3.4", "creatorName": "creator", "mailContent": "mail"})
ctf = get_ctf("ctfuser", "old", "mission")
print ctf.classID
print ctf.get_description
print ctf.get_template
print ctf.get_creator_name
print ctf.get_mail_content
print ctf.player_success
print ctf.player_success
print reset_ctf_password("new")
print get_ctf("ctfuser", "new", "mission") != null
"#;

    let (output, error) = run_code_in(code, true);

    assert_eq!(error, None);
    assert_eq!(output, "ctfEvent\ndesc\n1.2.3.4\ncreator\nmail\n1\n0\n1\n1");
}

#[tokio::test]
async fn should_preserve_ctf_accounts_events_and_completion() {
    let mut world = World::new(&PlayerOptions::default()).await.unwrap();
    let player = world.player();

    world
        .create_ctf_account(player, "ctfuser", "old")
        .await
        .unwrap();
    world
        .create_ctf_event(&CtfEvent {
            name: "mission".to_owned(),
            user: "ctfuser".to_owned(),
            password_hash: hash_password("old"),
            description: "desc".to_owned(),
            public_address: "1.2.3.4".to_owned(),
            creator: world.player_person(),
            creator_name: "creator".to_owned(),
            mail_content: "mail".to_owned(),
            creator_bank_user: "test".to_owned(),
            quality_rank: 3,
            difficulty_rank: 3,
            num_votes: 1,
            is_rated: false,
        })
        .await
        .unwrap();
    assert!(world.add_ctf_player("mission", player).await.unwrap());
    assert!(world.player_success("mission", player).await.unwrap());
    assert!(world.reset_ctf_password(player, "new").await.unwrap());

    let database = save_copy(&mut world).await;

    let restored = World::load(&database).await.unwrap();

    let event = restored.ctf_event("mission").await.unwrap().unwrap();

    assert_eq!(event.description, "desc");
    assert!(restored.ctf_players("mission").await.unwrap()[0].completed);
    assert_eq!(event.password_hash, hash_password("new"));
    assert_eq!(
        restored
            .ctf_account(player)
            .await
            .unwrap()
            .unwrap()
            .password_hash,
        hash_password("new")
    );
}
