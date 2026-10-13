use greyhack_world::{Email, Login};

use crate::support::npc_mail::{PASSWORD, inbox, inboxes, network};

#[tokio::test]
async fn should_generate_the_mails_of_an_npc_account_on_its_first_login_only() {
    let (mut world, _, members) = network().await;

    for member in &members {
        assert!(inbox(&world, &member.address).await.is_empty());
    }

    for member in &members {
        assert_eq!(
            world.mail_login(&member.address, PASSWORD).await.unwrap(),
            Login::Valid
        );
    }

    let first = inboxes(&world, &members).await;

    assert!(first.iter().any(|inbox| !inbox.is_empty()));

    for member in &members {
        world.mail_login(&member.address, PASSWORD).await.unwrap();
    }

    assert_eq!(inboxes(&world, &members).await, first);
}

#[tokio::test]
async fn should_mail_the_colleagues_the_npc_writes_to() {
    let (mut world, _, members) = network().await;
    let addresses: Vec<&str> = members
        .iter()
        .map(|member| member.address.as_str())
        .collect();

    for member in &members {
        world.mail_login(&member.address, PASSWORD).await.unwrap();
    }

    for (member, inbox) in members.iter().zip(inboxes(&world, &members).await) {
        for email in &inbox {
            if email.from.starts_with("no-reply@") {
                continue;
            }

            let other = &email.from;

            assert!(addresses.contains(&other.as_str()), "{other}");

            let counterpart = self::inbox(&world, other).await;
            let sent = email.body.starts_with("> ");

            assert!(
                counterpart.iter().any(|mail| mail.from == member.address
                    && mail.subject == email.subject
                    && mail.body.trim_start_matches("> ") == email.body.trim_start_matches("> ")
                    && mail.body.starts_with("> ") != sent),
                "{} has no counterpart of {:?}",
                other,
                email
            );
        }
    }
}

#[tokio::test]
async fn should_refuse_wrong_passwords_without_generating() {
    let (mut world, _, members) = network().await;

    for member in &members {
        assert_eq!(
            world.mail_login(&member.address, "wrong").await.unwrap(),
            Login::InvalidCredentials
        );
    }

    for inbox in inboxes(&world, &members).await {
        assert!(inbox.is_empty());
    }
}

#[tokio::test]
async fn should_not_generate_for_a_fired_npc_account() {
    let (mut world, public_ip, members) = network().await;
    let fired = &members[0];
    let user = world
        .npc(fired.person)
        .await
        .unwrap()
        .map(|npc| npc.user)
        .unwrap();

    world.fire_npc(&public_ip, &user).await.unwrap();

    assert_eq!(
        world.mail_login(&fired.address, PASSWORD).await.unwrap(),
        Login::AccountInactive
    );
    assert!(inbox(&world, &fired.address).await.is_empty());
}

#[tokio::test]
async fn should_generate_an_npc_account_before_a_mail_is_delivered_to_it() {
    let (mut world, _, members) = network().await;
    let mut generated = false;

    for member in &members {
        world
            .deliver_mail(
                &member.address,
                "player@home.com",
                "Hello",
                "From the player",
            )
            .await
            .unwrap()
            .unwrap();

        // The delivered mail is the newest, so it follows the generated mails.
        let inbox = inbox(&world, &member.address).await;

        assert_eq!(inbox[0].subject, "Hello");
        assert_eq!(inbox[0].from, "player@home.com");
        generated |= inbox.len() > 1;
    }

    assert!(generated);

    let delivered = inboxes(&world, &members).await;

    for member in &members {
        world.mail_login(&member.address, PASSWORD).await.unwrap();
    }

    assert_eq!(inboxes(&world, &members).await, delivered);
}

#[tokio::test]
async fn should_not_generate_for_accounts_without_an_npc() {
    let (mut world, _, _) = network().await;

    world
        .add_mail_account("player@home.com", PASSWORD)
        .await
        .unwrap();

    assert_eq!(
        world.mail_login("player@home.com", PASSWORD).await.unwrap(),
        Login::Valid
    );
    assert!(inbox(&world, "player@home.com").await.is_empty());
}

#[tokio::test]
async fn should_list_the_newest_mail_first() {
    let (mut world, _, _) = network().await;

    world
        .add_mail_account("player@home.com", PASSWORD)
        .await
        .unwrap();

    for subject in ["First", "Second", "Third"] {
        world
            .deliver_mail("player@home.com", "other@home.com", subject, "text")
            .await
            .unwrap();
    }

    let subjects: Vec<String> = inbox(&world, "player@home.com")
        .await
        .into_iter()
        .map(|email| email.subject)
        .collect();

    assert_eq!(subjects, ["Third", "Second", "First"]);
}

#[tokio::test]
async fn should_keep_the_generation_state_of_a_restored_account() {
    let (mut world, _, members) = network().await;
    let address = &members[1].address;
    let mail = Email {
        id: String::new(),
        from: "a@b.com".to_owned(),
        subject: "Thread".to_owned(),
        body: "first".to_owned(),
        replies: vec!["> second".to_owned()],
    };

    world
        .restore_mail_account(address, PASSWORD, true, std::slice::from_ref(&mail))
        .await
        .unwrap();
    world.mail_login(address, PASSWORD).await.unwrap();

    let stored = inbox(&world, address).await;

    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].replies, mail.replies);
    assert_eq!(stored[0].body, mail.body);
}
