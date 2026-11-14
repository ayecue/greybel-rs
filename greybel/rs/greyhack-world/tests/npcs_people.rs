use greyhack_behaviour::NetworkKind;

use crate::support::npcs_jobs::world_with;

/// A person takes the mail address the game gave them, which a mission NPC's is not `surname@domain`
/// (`NpcInfo.UpdateNpcInfo`'s `mailUserAlias`): their account is renamed with its password and mails.
#[tokio::test]
async fn should_rename_the_mail_account_of_a_person_with_its_mails() {
    let (world, network) = world_with(NetworkKind::Supermarket).await;
    let admin = world
        .modules
        .npcs
        .npcs
        .network_admin_person(network)
        .await
        .unwrap()
        .unwrap();
    let address = world
        .modules
        .npcs
        .mail
        .mail_address(admin)
        .await
        .unwrap()
        .unwrap();
    let alias = format!("Alias@{}", address.split('@').nth(1).unwrap());
    let before = world
        .modules
        .npcs
        .mail
        .mail_account(&address)
        .await
        .unwrap()
        .unwrap();

    world
        .modules
        .npcs
        .mail
        .insert_email(&address, "someone@mail.test", "Hello", "Body")
        .await
        .unwrap();
    world
        .modules
        .npcs
        .npcs
        .set_person_mail_address(admin, &alias)
        .await
        .unwrap();

    let after = world
        .modules
        .npcs
        .mail
        .mail_account(&alias)
        .await
        .unwrap()
        .unwrap();

    assert!(
        world
            .modules
            .npcs
            .mail
            .mail_account(&address)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(after.password_hash, before.password_hash);
    assert!(after.inbox.iter().any(|mail| mail.subject == "Hello"));
    assert_eq!(
        world.modules.npcs.mail.mail_address(admin).await.unwrap(),
        Some(alias)
    );
}

/// An NPC's mail address is its account's name unless the game gives it another (`NpcInfo.NpcInfo`,
/// `NpcInfo.UpdateNpcInfo`).
#[tokio::test]
async fn should_give_an_npc_the_mail_address_of_the_name_it_keeps() {
    let (world, network) = world_with(NetworkKind::Supermarket).await;
    let admin = world
        .modules
        .npcs
        .npcs
        .network_admin_person(network)
        .await
        .unwrap()
        .unwrap();
    let computer = world
        .modules
        .npcs
        .npcs
        .npc(admin)
        .await
        .unwrap()
        .unwrap()
        .computer;
    let address = world
        .modules
        .npcs
        .mail
        .mail_address(admin)
        .await
        .unwrap()
        .unwrap();
    let domain = address.split('@').nth(1).unwrap();
    let now = world.handle.time_seconds();

    for user in ["Plain", "Taker"] {
        world
            .modules
            .core
            .computers
            .users
            .add_user(computer, user, "pass1")
            .await
            .unwrap()
            .unwrap();
    }

    let plain = world
        .modules
        .npcs
        .npcs
        .add_npc(computer, "Plain", "Ann Plain", now)
        .await
        .unwrap()
        .unwrap();
    let taker = world
        .modules
        .npcs
        .npcs
        .add_npc_with_mail_name(computer, "Taker", "Replaced", "Bob Taker", now)
        .await
        .unwrap()
        .unwrap();

    for (person, expected) in [(plain, "Plain"), (taker, "Replaced")] {
        let address = world
            .modules
            .npcs
            .mail
            .mail_address(person)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(address, format!("{expected}@{domain}"));
        assert!(
            world
                .modules
                .npcs
                .mail
                .mail_account(&address)
                .await
                .unwrap()
                .is_some()
        );
    }
}
