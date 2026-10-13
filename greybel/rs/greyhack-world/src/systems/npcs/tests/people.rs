use greyhack_behaviour::NetworkKind;

use crate::systems::npcs::jobs::tests::support::world_with;

/// A person takes the mail address the game gave them, which a mission NPC's is not `surname@domain`
/// (`NpcInfo.UpdateNpcInfo`'s `mailUserAlias`): their account is renamed with its password and mails.
#[tokio::test]
async fn should_rename_the_mail_account_of_a_person_with_its_mails() {
    let (mut world, network) = world_with(NetworkKind::Supermarket).await;
    let admin = world
        .db
        .network_admin_person(network)
        .await
        .unwrap()
        .unwrap();
    let address = world
        .person(admin)
        .await
        .unwrap()
        .unwrap()
        .mail_address
        .unwrap();
    let alias = format!("Alias@{}", address.split('@').nth(1).unwrap());
    let before = world.mail_account(&address).await.unwrap().unwrap();

    world
        .db
        .insert_email(&address, "someone@mail.test", "Hello", "Body")
        .await
        .unwrap();
    world.set_person_mail_address(admin, &alias).await.unwrap();

    let after = world.mail_account(&alias).await.unwrap().unwrap();

    assert!(world.mail_account(&address).await.unwrap().is_none());
    assert_eq!(after.password_hash, before.password_hash);
    assert!(after.inbox.iter().any(|mail| mail.subject == "Hello"));
    assert_eq!(
        world.person(admin).await.unwrap().unwrap().mail_address,
        Some(alias)
    );
}

/// An NPC's mail address is its account's name unless the game gives it another (`NpcInfo.NpcInfo`,
/// `NpcInfo.UpdateNpcInfo`).
#[tokio::test]
async fn should_give_an_npc_the_mail_address_of_the_name_it_keeps() {
    let (mut world, network) = world_with(NetworkKind::Supermarket).await;
    let admin = world
        .db
        .network_admin_person(network)
        .await
        .unwrap()
        .unwrap();
    let computer = world.db.npc(admin).await.unwrap().unwrap().computer;
    let address = world
        .person(admin)
        .await
        .unwrap()
        .unwrap()
        .mail_address
        .unwrap();
    let domain = address.split('@').nth(1).unwrap();
    let now = world.time_seconds();

    for user in ["Plain", "Taker"] {
        world
            .add_user(computer, user, "pass1")
            .await
            .unwrap()
            .unwrap();
    }

    let plain = world
        .add_npc(computer, "Plain", "Ann Plain", now)
        .await
        .unwrap()
        .unwrap();
    let taker = world
        .add_npc_with_mail_name(computer, "Taker", "Replaced", "Bob Taker", now)
        .await
        .unwrap()
        .unwrap();

    for (person, expected) in [(plain, "Plain"), (taker, "Replaced")] {
        let address = world
            .person(person)
            .await
            .unwrap()
            .unwrap()
            .mail_address
            .unwrap();

        assert_eq!(address, format!("{expected}@{domain}"));
        assert!(world.mail_account(&address).await.unwrap().is_some());
    }
}
