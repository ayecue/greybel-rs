#[path = "support/mod.rs"]
mod support;

use mono_random::Random;
use support::{all, boss, colleague, network, owner, run, weak_admin};

use super::draw::{BREAK_SUBJECTS, SPAM_SUBJECTS, STARTER_MISSION_SUBJECT};
use super::{Administrator, Delivery, Owner, contents};
use crate::{CompanyAlign, JobRole, NetworkAdmin, WorkPerformance};

#[test]
fn should_mail_the_owner_about_a_poorly_or_highly_performing_colleague() {
    let network = network();
    let owner = owner(&network);
    let runs = all(&owner, &network, None);
    let work: Vec<&Delivery> = runs
        .iter()
        .flatten()
        .filter(|delivery| delivery.subject.starts_with("about First"))
        .collect();

    assert!(!work.is_empty());

    for delivery in work {
        assert!(
            ["about Firstbad", "about Firstgood"].contains(&delivery.subject.as_str()),
            "{}",
            delivery.subject
        );
        // `$NPC` stands for the first name of the colleague the mail is about.
        assert!(delivery.text.starts_with("<WORKMAIL_"));
    }
}

#[test]
fn should_not_write_about_administrators() {
    let network = network();
    let owner = owner(&network);

    for delivery in all(&owner, &network, None).iter().flatten() {
        assert_ne!(delivery.subject, "about Firstboss");
    }
}

#[test]
fn should_deliver_work_mails_to_both_mailboxes_with_opposite_directions() {
    let network = network();
    let owner = owner(&network);

    for deliveries in all(&owner, &network, None) {
        for pair in deliveries.chunks(2) {
            let [own, other] = pair else {
                continue;
            };

            if !own.subject.starts_with("about First") {
                continue;
            }

            assert_eq!(own.to, owner.colleague.address);
            assert_eq!(other.other, own.to);
            assert_eq!(own.other, other.to);
            assert_eq!(own.text, other.text);
            assert_ne!(own.sending, other.sending);
            assert!(own.other != owner.colleague.address);
        }
    }
}

#[test]
fn should_pick_the_work_mail_text_from_fourteen_variants() {
    let network = network();
    let owner = owner(&network);
    let mut numbers = std::collections::BTreeSet::new();

    for delivery in all(&owner, &network, None).iter().flatten() {
        if let Some(number) = delivery
            .text
            .strip_prefix("<WORKMAIL_GOOD_")
            .or_else(|| delivery.text.strip_prefix("<WORKMAIL_BAD_"))
        {
            numbers.insert(number.trim_end_matches('>').parse::<usize>().unwrap());
        }
    }

    assert_eq!(numbers.first(), Some(&1));
    assert_eq!(numbers.last(), Some(&14));
}

#[test]
fn should_skip_the_work_mail_without_a_colleague_of_the_drawn_performance() {
    let network = vec![
        colleague("owner", JobRole::Sales, WorkPerformance::Normal),
        colleague("fine", JobRole::Sales, WorkPerformance::Good),
        colleague("boss", JobRole::Admin, WorkPerformance::Excellent),
    ];
    let owner = owner(&network);

    for delivery in all(&owner, &network, None).iter().flatten() {
        assert!(!delivery.subject.starts_with("about First"));
    }
}

#[test]
fn should_send_the_break_mail_with_the_owners_break_time() {
    let network = network();
    let owner = owner(&network);
    let mails: Vec<Delivery> = all(&owner, &network, None)
        .into_iter()
        .flatten()
        .filter(|delivery| delivery.text.starts_with("<WORKMAIL_BREAK_"))
        .collect();

    assert!(!mails.is_empty());

    for pair in mails.chunks(2) {
        let [own, other] = pair else {
            panic!("a break mail goes to two mailboxes");
        };

        assert!(own.sending && !other.sending);
        assert!(BREAK_SUBJECTS.contains(&own.subject.as_str()));
        assert_eq!(own.subject, other.subject);
        assert_eq!(own.to, owner.colleague.address);
    }
}

#[test]
fn should_format_the_break_time_in_the_game_clock() {
    let network = network();
    let mut owner = owner(&network);

    owner.break_start = 24 * 3_600 + 75 * 60;

    let text = |key: &str| {
        if key.starts_with("WORKMAIL_BREAK_") {
            "at $TIME".to_owned()
        } else {
            key.to_owned()
        }
    };
    let found = (0..400).find_map(|seed| {
        contents(&mut Random::new(seed), &owner, &network, None, None, &text)
            .into_iter()
            .find(|delivery| delivery.text.starts_with("at "))
    });

    // `TimeSpan.Hours` of 25:15 is 1.
    assert_eq!(
        found.map(|delivery| delivery.text),
        Some("at 01:15".to_owned())
    );
}

#[test]
fn should_deliver_up_to_three_spam_mails_from_no_reply_addresses() {
    let network = network();
    let owner = owner(&network);
    let mut counts = std::collections::BTreeSet::new();

    for deliveries in all(&owner, &network, None) {
        let spam: Vec<&Delivery> = deliveries
            .iter()
            .filter(|delivery| delivery.other.starts_with("no-reply@"))
            .collect();

        counts.insert(spam.len());

        for delivery in spam {
            assert!(SPAM_SUBJECTS.contains(&delivery.subject.as_str()));
            assert!(delivery.other.ends_with(".com"));
            assert!(delivery.text.starts_with("<WORKMAIL_SPAM_"));
            assert_eq!(delivery.to, owner.colleague.address);
            assert!(!delivery.sending);
        }
    }

    assert_eq!(counts.into_iter().collect::<Vec<_>>(), [0, 1, 2, 3]);
}

#[test]
fn should_only_write_zero_day_mails_for_aligned_owners() {
    let network = network();
    let plain = owner(&network);
    let aligned = Owner {
        company_align: CompanyAlign::Bad,
        day_zero_contact: Some("good@corp.com".to_owned()),
        ..owner(&network)
    };

    for deliveries in all(&plain, &network, None) {
        assert!(
            deliveries
                .iter()
                .all(|delivery| delivery.subject != "About the company")
        );
    }

    for deliveries in all(&aligned, &network, None) {
        let own = deliveries
            .iter()
            .find(|delivery| {
                delivery.subject == "About the company" && delivery.to == aligned.colleague.address
            })
            .expect("an aligned NPC always receives the zero-day mail");
        let contact = deliveries
            .iter()
            .find(|delivery| {
                delivery.subject == "About the company" && delivery.to == "good@corp.com"
            })
            .expect("the contact receives it too");

        assert!(own.text.starts_with("<WORKMAIL_0D_BAD_"));
        assert_eq!(own.text, contact.text);
        assert_eq!(own.other, "good@corp.com");
        assert!(own.sending && !contact.sending);
        // The zero-day mail is the last the game puts into the mailbox.
        assert_eq!(
            deliveries.last().map(|delivery| delivery.to.as_str()),
            Some("good@corp.com")
        );
    }
}

#[test]
fn should_write_about_the_administrator_by_the_administrators_performance() {
    let network = network();
    let owner = owner(&network);
    let bad = boss(weak_admin());
    let good = boss(NetworkAdmin {
        work_performance: WorkPerformance::VeryGood,
        ..weak_admin()
    });
    let normal = boss(NetworkAdmin {
        work_performance: WorkPerformance::Normal,
        ..weak_admin()
    });
    let super_admin = boss(NetworkAdmin {
        intelligence: 9,
        pc_experience: 9,
        work_performance: WorkPerformance::Good,
        dead_or_fired: true,
    });
    let kinds = |admin: &Administrator| -> std::collections::BTreeSet<String> {
        all(&owner, &network, Some(admin))
            .into_iter()
            .flatten()
            .filter(|delivery| delivery.subject == "about the admin")
            .map(|delivery| delivery.text.rsplit_once('_').unwrap().0.to_owned())
            .collect()
    };

    assert_eq!(kinds(&bad), ["<WORKMAIL_ADMIN_BAD".to_owned()].into());
    assert_eq!(kinds(&good), ["<WORKMAIL_ADMIN_GOOD".to_owned()].into());
    assert!(kinds(&normal).is_empty());
    // A dead or fired administrator still counts as super (`IsSuperAdmin` without `checkDeadOrFired`).
    assert_eq!(
        kinds(&super_admin),
        ["<WORKMAIL_ADMIN_SUPER".to_owned()].into()
    );
}

#[test]
fn should_not_write_about_the_administrator_to_the_administrator() {
    let network = network();
    let admin_owner = Owner {
        colleague: network[3].clone(),
        ..owner(&network)
    };

    for delivery in all(&admin_owner, &network, Some(&boss(weak_admin())))
        .iter()
        .flatten()
    {
        assert_ne!(delivery.subject, "about the admin");
    }
}

#[test]
fn should_draw_the_same_mails_for_the_same_seed() {
    let network = network();
    let owner = owner(&network);

    assert_eq!(
        run(7, &owner, &network, None),
        run(7, &owner, &network, None)
    );
}

#[test]
fn should_write_about_the_mission_contact_in_most_mission_networks() {
    let network = network();
    let owner = owner(&network);
    let mut networks = 0;

    for seed in 0..200 {
        let mails: Vec<Delivery> = contents(
            &mut Random::new(seed),
            &owner,
            &network,
            None,
            Some("10.0.0.7"),
            &support::text,
        )
        .into_iter()
        .filter(|delivery| delivery.subject == STARTER_MISSION_SUBJECT)
        .collect();

        if mails.is_empty() {
            continue;
        }

        networks += 1;

        let [own, other] = mails.as_slice() else {
            panic!("a starter mission mail goes to two mailboxes");
        };

        assert_eq!(own.to, owner.colleague.address);
        assert_eq!(own.sending, !other.sending);
        assert!(own.text.starts_with("<WORKMAIL_MISSIONS_"));
        assert_eq!(own.text, other.text);
    }

    // `random.Next(5) > 0`: four of five.
    assert!((140..=180).contains(&networks), "{networks} of 200");
}

#[test]
fn should_not_write_about_a_mission_without_one() {
    let network = network();
    let owner = owner(&network);

    for deliveries in all(&owner, &network, None) {
        assert!(
            deliveries
                .iter()
                .all(|delivery| delivery.subject != STARTER_MISSION_SUBJECT)
        );
    }
}

#[test]
fn should_have_the_seven_starter_mission_texts_with_the_contact() {
    let texts = greyhack_world_assets::GameTextTable::english();

    for number in 1..=7 {
        let text = texts
            .get(&format!("WORKMAIL_MISSIONS_{number}"))
            .unwrap_or_else(|| panic!("WORKMAIL_MISSIONS_{number}"));

        assert!(
            text.contains("$LOCAL_IP"),
            "WORKMAIL_MISSIONS_{number}: {text}"
        );
    }
}
